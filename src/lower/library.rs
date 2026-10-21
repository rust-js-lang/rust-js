//! Translate checked Rust identities to the library contract (ADR 0100): the
//! items a library exports, and the ones a consumer imports.

use super::{FnInfo, module_path};
use crate::library::{Dependencies, Imported, Item, Library, VERSION};
use rustc_middle::ty::TyCtxt;
use rustc_span::def_id::{DefId, LOCAL_CRATE};
use std::collections::{BTreeSet, HashMap, HashSet};

/// What a library and its consumers both call an item: rustc's `DefPathHash`
/// of it, the same in every crate that sees it.
fn key(tcx: TyCtxt<'_>, id: DefId) -> String {
    tcx.def_path_hash(id).0.to_hex()
}

/// Can another crate reach `id`, a local item? rustc's own answer, which
/// follows `pub use`; and every trait impl, and its methods: the library's
/// generic code may ask a consumer that instantiates it for an impl of a
/// type of its own it can't name, base64's `Utf8SingleCodeUnitWriter`'s
/// `io::Write`, by a bound (ADR 0100).
pub(super) fn reachable(tcx: TyCtxt<'_>, id: DefId) -> bool {
    let id = tcx.trait_impl_of_assoc(id).unwrap_or(id);
    matches!(tcx.def_kind(id), rustc_hir::def::DefKind::Impl { of_trait: true })
        || id
            .as_local()
            .is_some_and(|local| tcx.effective_visibilities(()).is_reachable(local))
}

/// What each function's contract says, of what a consumer can't work out
/// from its types: the drops it takes (ADR 0098), whether it may fail
/// (ADR 0187), the drops it takes none of (ADR 0190), and what it asks of
/// its type parameters (ADR 0145).
pub(super) struct Contracts<'a> {
    pub(super) drop_params: &'a HashMap<DefId, Vec<u32>>,
    pub(super) failing: &'a HashSet<DefId>,
    pub(super) no_drops: &'a HashMap<DefId, BTreeSet<u32>>,
    pub(super) type_facts: &'a HashMap<DefId, Vec<(u32, super::recognition::TypeFact)>>,
    /// Those that take their caller's location (ADR 0427).
    pub(super) caller_locations: &'a HashSet<DefId>,
    /// The type parameters each steps as they are, by which trait (ADRs
    /// 0411, 0548).
    pub(super) stepped: &'a HashSet<(DefId, u32, DefId)>,
    /// Whether it keeps a `Vec`'s capacity, which its consumers then do
    /// too, as what they give it may be asked (ADR 0419).
    pub(super) keeps_capacity: bool,
    /// Whether its writers take their `Formatter`'s options (ADR 0137).
    pub(super) writers_take_options: bool,
}

/// The items another crate can reach, and the impls it calls through them.
pub(super) fn exports(
    tcx: TyCtxt<'_>,
    functions: &HashMap<DefId, FnInfo>,
    contracts: &Contracts<'_>,
    trait_impls: &[DefId],
    dependencies: &Dependencies,
) -> Library {
    let Contracts {
        drop_params,
        failing,
        no_drops,
        type_facts,
        caller_locations,
        stepped,
        keeps_capacity,
        writers_take_options,
    } = contracts;
    let crate_name = tcx.crate_name(LOCAL_CRATE);
    let mut items: Vec<_> = functions
        .iter()
        .filter(|&(&id, _)| reachable(tcx, id))
        .map(|(&id, info)| Item {
            key: key(tcx, id),
            rust_path: format!("{crate_name}::{}", tcx.def_path_str(id)),
            module: module_path(tcx, info.module),
            export: info.owner.clone().unwrap_or_else(|| info.name.clone()),
            member: info.owner.as_ref().map(|_| info.name.clone()),
            drops: drop_params.get(&id).cloned().unwrap_or_default(),
            fails: failing.contains(&id),
            no_drops: no_drops
                .get(&id)
                .map(|params| params.iter().copied().collect())
                .unwrap_or_default(),
            type_facts: (type_facts.get(&id).into_iter().flatten())
                .map(|&(index, fact)| (index, fact.word().to_string()))
                .collect(),
            takes_location: caller_locations.contains(&id),
            stepped: {
                let mut by: Vec<(u32, String)> = (stepped.iter())
                    .filter(|&&(of, _, _)| of == id)
                    .map(|&(_, index, by)| (index, key(tcx, by)))
                    .collect();
                by.sort();
                by
            },
        })
        .collect();
    items.sort_by(|a, b| a.rust_path.cmp(&b.rust_path).then_with(|| a.key.cmp(&b.key)));
    let mut impls: Vec<String> = trait_impls
        .iter()
        .filter(|&&id| reachable(tcx, id))
        .map(|&id| key(tcx, id))
        .collect();
    impls.sort();
    Library {
        version: VERSION,
        name: crate_name.to_string(),
        crate_hash: tcx.crate_hash(LOCAL_CRATE).to_string(),
        items,
        impls,
        libraries: {
            let mut names: Vec<String> = dependencies.libraries.keys().cloned().collect();
            names.sort();
            names
        },
        inputs: Vec::new(),
        keeps_capacity: *keeps_capacity,
        writers_take_options: *writers_take_options,
    }
}

/// What this crate's dependencies export, looked up by the `DefId`s rustc
/// gives their items here.
pub(super) struct Foreign<'a, 'tcx> {
    tcx: TyCtxt<'tcx>,
    dependencies: &'a Dependencies,
}

impl<'a, 'tcx> Foreign<'a, 'tcx> {
    pub(super) fn new(tcx: TyCtxt<'tcx>, dependencies: &'a Dependencies) -> Self {
        Foreign { tcx, dependencies }
    }

    /// Is each library's JS from the crate rustc loaded the metadata of? A
    /// library rebuilt since its JS was, or metadata from another build of it,
    /// isn't: its JS would be for another crate than the one this was checked
    /// against.
    pub(super) fn check(&self) -> bool {
        let mut matched = true;
        for &krate in self.tcx.crates(()) {
            let name = self.tcx.crate_name(krate);
            if let Some(library) = self.dependencies.libraries.get(name.as_str())
                && self.tcx.crate_hash(krate).to_string() != library.crate_hash
            {
                self.tcx.dcx().err(format!(
                    "rust-js: the metadata rustc loaded for `{name}` isn't of the build its JS was made from: rebuild `{name}`, and use the metadata rust-js writes beside its JS"
                ));
                matched = false;
            }
        }
        matched
    }

    /// Is `id` a dependency's, one rust-js compiled as a library?
    pub(super) fn in_library(&self, id: DefId) -> bool {
        !id.is_local()
            && self
                .dependencies
                .libraries
                .contains_key(self.tcx.crate_name(id.krate).as_str())
    }

    /// The item `id` is in its library's JS, if the library exports it.
    /// What a library's item asks of its type parameters (ADR 0145).
    pub(super) fn type_facts(&self, id: DefId) -> Vec<(u32, super::recognition::TypeFact)> {
        (self.item(id).into_iter().flat_map(|item| &item.type_facts))
            .filter_map(|(index, word)| super::recognition::TypeFact::of_word(word).map(|fact| (*index, fact)))
            .collect()
    }

    pub(super) fn item(&self, id: DefId) -> Option<&'a Imported> {
        if id.is_local() {
            return None;
        }
        let library = self
            .dependencies
            .libraries
            .get(self.tcx.crate_name(id.krate).as_str())?;
        library.items.get(&key(self.tcx, id))
    }

    /// Why `id` can't be used, if it's a library's its manifest doesn't list.
    pub(super) fn unlisted(&self, id: DefId) -> Option<String> {
        (self.in_library(id) && self.item(id).is_none()).then(|| {
            format!(
                "rust-js: `{}` isn't one of the items `{}`'s manifest exports",
                self.tcx.def_path_str(id),
                self.tcx.crate_name(id.krate)
            )
        })
    }

    /// May `id`, a library's, return `Err(fmt::Error)` (ADR 0187)?
    pub(super) fn fails(&self, id: DefId) -> bool {
        self.item(id).is_some_and(|item| item.fails)
    }

    /// Does a library's `id` step its type parameter `index` as it is, by
    /// `by`, given its dictionary (ADRs 0411, 0548)?
    pub(super) fn steps(&self, id: DefId, index: u32, by: DefId) -> bool {
        let by = key(self.tcx, by);
        self.item(id)
            .is_some_and(|item| item.stepped.iter().any(|(at, of)| *at == index && *of == by))
    }

    /// Does a library's `id` take its caller's location (ADR 0427)?
    pub(super) fn takes_location(&self, id: DefId) -> bool {
        self.item(id).is_some_and(|item| item.takes_location)
    }

    /// Does any of the libraries keep a `Vec`'s capacity (ADR 0419)?
    pub(super) fn keeps_capacity(&self) -> bool {
        self.dependencies
            .libraries
            .values()
            .any(|library| library.keeps_capacity)
    }

    /// Do the writers of `krate`, a library's, take their `Formatter`'s
    /// options (ADR 0137)?
    pub(super) fn writers_take_options(&self, krate: rustc_span::def_id::CrateNum) -> bool {
        (self.dependencies.libraries.get(self.tcx.crate_name(krate).as_str()))
            .is_some_and(|library| library.writers_take_options)
    }

    /// May any of the libraries' items?
    pub(super) fn any_fails(&self) -> bool {
        self.all().any(|item| item.fails)
    }

    /// Is `id` a library's trait impl, one its consumers call?
    pub(super) fn has_impl(&self, id: DefId) -> bool {
        !id.is_local()
            && self
                .dependencies
                .libraries
                .get(self.tcx.crate_name(id.krate).as_str())
                .is_some_and(|library| library.impls.contains(&key(self.tcx, id)))
    }

    /// Does this crate use a library?
    pub(super) fn any(&self) -> bool {
        !self.dependencies.libraries.is_empty()
    }

    /// Every item the dependencies export, as the imports they'd be.
    pub(super) fn all(&self) -> impl Iterator<Item = &'a Imported> {
        self.dependencies
            .libraries
            .values()
            .flat_map(|library| library.items.values())
    }
}
