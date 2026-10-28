//! Translate checked Rust identities to the library contract (ADR 0100): the
//! items a library exports, and the ones a consumer imports.

use super::bindings::HeldBinding;
use super::{FnInfo, module_path};
use crate::library::{Dependencies, Held, HeldImport, Imported, Item, ItemFact, Library, RcShape, VERSION};
use rustc_middle::ty::TyCtxt;
use rustc_span::def_id::{DefId, LOCAL_CRATE, LocalModId};
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
    // A std body compiled into `@rust-js/std` is what its consumers call.
    if super::recognition::is_compiled_std(tcx, id) {
        return true;
    }
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
pub(super) struct Contracts<'a, 'tcx> {
    pub(super) drop_params: &'a HashMap<DefId, Vec<u32>>,
    /// The type parameters whose `Copy` bound takes a copy function (ADRs
    /// 0289, 0914), whose `Clone` bound takes its dictionary (ADR 0914,
    /// amended), and which are stepped by a trait (ADR 0548).
    pub(super) given: super::traits::Given<'a>,
    pub(super) failing: &'a HashSet<DefId>,
    pub(super) no_drops: &'a HashMap<DefId, BTreeSet<u32>>,
    pub(super) type_facts: &'a HashMap<DefId, Vec<(u32, super::recognition::TypeFact)>>,
    /// Those that take their caller's location (ADR 0427).
    pub(super) caller_locations: &'a HashSet<DefId>,
    /// Whether it keeps a `Vec`'s capacity, which its consumers then do
    /// too, as what they give it may be asked (ADR 0419).
    pub(super) keeps_capacity: bool,
    /// Whether its writers take their `Formatter`'s options (ADR 0137).
    pub(super) writers_take_options: bool,
    /// Whether it makes a `dyn Any` of what drops (ADR 0691, amended).
    pub(super) makes_dropping_anys: bool,
    /// Whether it makes a `dyn Fn` of what drops (ADR 0626, amended).
    pub(super) makes_dropping_fn_dyns: bool,
    /// What the `Rc`s it counts point at (ADR 0852).
    pub(super) counted: &'a super::analysis::Counted<'tcx>,
    /// What it does with its `RefCell`s (ADR 0863).
    pub(super) ref_cells: crate::library::RefCellFacts,
    /// What it says of its structs' fields (ADRs 0863, amended, 0906).
    pub(super) fields: Vec<crate::library::Field>,
    /// The thread-locals made where they're first used (ADR 0037, amended).
    pub(super) lazy_locals: &'a HashSet<rustc_span::def_id::LocalDefId>,
    /// The exported thread-locals and `static mut`s that are their module's
    /// variables (ADRs 0270, 0096, amended).
    pub(super) variables: &'a HashSet<DefId>,
    /// Its types equal only to themselves (ADR 0864).
    pub(super) identities: &'a [DefId],
    /// Its functions each call of which is what it's given (ADR 0584,
    /// amended).
    pub(super) gives_argument: &'a [DefId],
}

/// What `id` asks of each of its type parameters that it asks anything of,
/// by index, as its contract says them: a `dropT`, no destructor, type
/// facts, and the traits it steps one by.
fn type_params(tcx: TyCtxt<'_>, id: DefId, contracts: &Contracts<'_, '_>) -> Vec<crate::library::TypeParam> {
    use crate::library::{ParamFact, TypeParam};
    let mut params: std::collections::BTreeMap<u32, TypeParam> = Default::default();
    fn param(params: &mut std::collections::BTreeMap<u32, TypeParam>, index: u32) -> &mut TypeParam {
        params.entry(index).or_insert_with(|| TypeParam {
            index,
            ..Default::default()
        })
    }
    for &index in contracts.drop_params.get(&id).into_iter().flatten() {
        param(&mut params, index).marks.insert(ParamFact::Drop);
    }
    for &(_, index) in contracts.given.copied.iter().filter(|&&(of, _)| of == id) {
        param(&mut params, index).marks.insert(ParamFact::Copy);
    }
    for &(_, index) in contracts.given.clones.iter().filter(|&&(of, _)| of == id) {
        param(&mut params, index).marks.insert(ParamFact::Clone);
    }
    // As its JS takes them, a function JS calls each as JS gives it (ADR
    // 0051, amended).
    for index in contracts.given.js_given.main(tcx, id).unboxed {
        param(&mut params, index).marks.insert(ParamFact::Unboxed);
    }
    for &index in contracts.no_drops.get(&id).into_iter().flatten() {
        param(&mut params, index).marks.insert(ParamFact::NoDrop);
    }
    for &(index, fact) in contracts.type_facts.get(&id).into_iter().flatten() {
        param(&mut params, index).facts.push(fact.word().to_string());
    }
    for &(_, index, by) in contracts.given.stepped.iter().filter(|&&(of, _, _)| of == id) {
        param(&mut params, index).stepped.push(key(tcx, by));
    }
    let mut params: Vec<_> = params.into_values().collect();
    for param in &mut params {
        param.stepped.sort();
    }
    params
}

/// Is `id` the crate's own, or std's compiled into `@rust-js/std` (ADR
/// 0561): not a library's it compiled itself, which is that library's (ADR
/// 0914).
fn own(tcx: TyCtxt<'_>, id: DefId) -> bool {
    id.is_local() || super::recognition::is_std_crate(tcx, id)
}

/// What a crate's JS exports of its own, each by its module's path and its
/// name there (`exported_names`).
pub(super) type Exported = HashSet<(Vec<String>, String)>;

/// What each of `modules`' JS exports of its own, by its module's path and
/// its name there: an item `export`ed, its default and those it exports by
/// another name.
pub(super) fn exported_names(modules: &[crate::program::UnlinkedModule]) -> Exported {
    let mut exported = HashSet::new();
    for module in modules.iter().map(|unlinked| &unlinked.module) {
        let mut export = |name: &str| exported.insert((module.path.clone(), name.to_string()));
        for item in &module.items {
            match item {
                crate::js::Item::Function(function) if function.export => export(&function.name),
                crate::js::Item::Const(constant) if constant.export => export(&constant.name),
                crate::js::Item::Namespace(namespace) if namespace.export => export(&namespace.name),
                _ => false,
            };
        }
        for name in module
            .default_export
            .iter()
            .chain(module.aliases.iter().map(|(local, _)| local))
        {
            export(name);
        }
    }
    exported
}

/// The items another crate can reach that its JS exports, `exported`
/// (`exported_names`), and the impls it calls through them; and the
/// bindings its modules hold, which a consumer naming one through its
/// module reads through it (ADR 0633). What a consumer reaches that it
/// doesn't export, the consumer compiles (ADR 0914, amended).
pub(super) fn exports<'tcx>(
    tcx: TyCtxt<'tcx>,
    (functions, exported): (&HashMap<DefId, FnInfo>, &Exported),
    contracts: &Contracts<'_, 'tcx>,
    (trait_impls, held): (&[DefId], &HashMap<LocalModId, Vec<HeldBinding>>),
    dependencies: &Dependencies,
) -> Library {
    let Contracts {
        // What each type parameter is asked, `type_params`'s.
        drop_params: _,
        no_drops: _,
        type_facts: _,
        given: _,
        failing,
        caller_locations,
        keeps_capacity,
        writers_take_options,
        makes_dropping_anys,
        makes_dropping_fn_dyns,
        counted,
        ref_cells: _,
        fields: _,
        lazy_locals: _,
        variables,
        identities,
        gives_argument,
    } = contracts;
    let crate_name = tcx.crate_name(LOCAL_CRATE);
    let mut items: Vec<_> = functions
        .iter()
        .filter(|&(&id, info)| {
            let name = info.owner.as_ref().unwrap_or(&info.name);
            own(tcx, id) && reachable(tcx, id) && exported.contains(&(module_path(tcx, info.module), name.clone()))
        })
        .map(|(&id, info)| Item {
            key: key(tcx, id),
            rust_path: format!("{crate_name}::{}", tcx.def_path_str(id)),
            module: module_path(tcx, info.module),
            export: info.owner.clone().unwrap_or_else(|| info.name.clone()),
            member: info.owner.as_ref().map(|_| info.name.clone()),
            // As its own crate imports it (ADRs 0596, 0611, 0773).
            exported_as: (info.owner.is_none())
                .then(|| super::default_exports::exported_as(tcx, id, &info.name))
                .filter(|exported| *exported != info.name),
            calling: crate::library::Calling {
                params: type_params(tcx, id, contracts),
                facts: [
                    // An impl's dictionary, of a method that may fail, which a
                    // consumer calls through it (`Foreign::through_dictionary`).
                    (
                        ItemFact::Fails,
                        failing.contains(&id)
                            || (matches!(tcx.def_kind(id), rustc_hir::def::DefKind::Impl { of_trait: true })
                                && (tcx.associated_item_def_ids(id).iter()).any(|method| failing.contains(method))),
                    ),
                    (ItemFact::TakesLocation, caller_locations.contains(&id)),
                    (ItemFact::Rest, super::bindings::takes_rest(tcx, id)),
                    (
                        ItemFact::Lazy,
                        id.as_local()
                            .is_some_and(|local| contracts.lazy_locals.contains(&local)),
                    ),
                    (ItemFact::Variable, variables.contains(&id)),
                ]
                .into_iter()
                .filter_map(|(fact, holds)| holds.then_some(fact))
                .collect(),
            },
        })
        .collect();
    items.sort_by(|a, b| a.rust_path.cmp(&b.rust_path).then_with(|| a.key.cmp(&b.key)));
    let mut impls: Vec<String> = trait_impls
        .iter()
        .filter(|&&id| own(tcx, id) && reachable(tcx, id))
        .map(|&id| key(tcx, id))
        .collect();
    impls.sort();
    let mut held: Vec<Held> = (held.iter())
        .flat_map(|(&module, bindings)| bindings.iter().map(move |binding| (module, binding)))
        .filter_map(|(module, binding)| {
            Some(Held {
                binding: key(tcx, binding.binding),
                holder: key(tcx, module.to_def_id()),
                rust_name: binding.ident.to_string(),
                module: module_path(tcx, module),
                export: binding.import()?.to_string(),
                name: binding.name.clone(),
            })
        })
        .collect();
    held.sort_by(|a, b| (&a.module, &a.rust_name).cmp(&(&b.module, &b.rust_name)));
    Library {
        version: VERSION,
        name: crate_name.to_string(),
        crate_hash: tcx.crate_hash(LOCAL_CRATE).to_string(),
        items,
        impls,
        // Those its consumers must be given too: not `@rust-js/std`, built
        // into the compiler they use (ADR 0561).
        libraries: {
            let mut names: Vec<String> = (dependencies.libraries.iter())
                .filter(|(_, library)| !library.builtin)
                .map(|(name, _)| name.clone())
                .collect();
            names.sort();
            names
        },
        inputs: Vec::new(),
        keeps_capacity: *keeps_capacity,
        writers_take_options: *writers_take_options,
        makes_dropping_anys: *makes_dropping_anys,
        makes_dropping_fn_dyns: *makes_dropping_fn_dyns,
        held,
        counted_rcs: {
            let mut shapes: Vec<RcShape> = match counted.all {
                true => vec![RcShape {
                    of: None,
                    args: Vec::new(),
                }],
                false => counted.types.iter().map(|&ty| rc_shape(tcx, ty)).collect(),
            };
            shapes.sort();
            shapes.dedup();
            shapes
        },
        ref_cells: contracts.ref_cells.clone(),
        fields: contracts.fields.clone(),
        identities: {
            let mut keys: Vec<String> = (identities.iter())
                .filter(|&&id| reachable(tcx, id))
                .map(|&id| key(tcx, id))
                .collect();
            keys.sort();
            keys
        },
        gives_argument: {
            let mut keys: Vec<String> = (gives_argument.iter())
                .filter(|&&id| reachable(tcx, id))
                .map(|&id| key(tcx, id))
                .collect();
            keys.sort();
            keys
        },
        types: declared_types(tcx),
        // Its package, which artifact planning reads (ADR 1008).
        package: None,
    }
}

/// Each type its modules' declarations export by its name, which a
/// consumer's declarations import (ADR 0210, amended): a public struct, enum
/// or union of a module's own that its declarations declare
/// (`declarations::declares_type`), its default too, sorted.
fn declared_types(tcx: TyCtxt<'_>) -> Vec<crate::library::DeclaredType> {
    let mut types: Vec<crate::library::DeclaredType> = (tcx.hir_crate_items(()).definitions())
        .map(|local| local.to_def_id())
        .filter(|&id| {
            super::declarations::declares_type(tcx, id)
                && matches!(tcx.def_kind(tcx.parent(id)), rustc_hir::def::DefKind::Mod)
                && tcx.visibility(id).is_public()
                && !super::bindings::is_mark(tcx, id)
        })
        .map(|id| crate::library::DeclaredType {
            key: key(tcx, id),
            module: module_path(tcx, tcx.parent_module_from_def_id(id.expect_local())),
            name: tcx.item_name(id).to_string(),
            default: super::default_exports::exported_as(tcx, id, tcx.item_name(id).as_str()) == "default",
        })
        .collect();
    types.sort();
    types
}

/// What a library and its consumers both call `id`, an item or a field
/// (`key`): a struct's field its contract names (ADR 0906).
pub(in crate::lower) fn item_key(tcx: TyCtxt<'_>, id: DefId) -> String {
    key(tcx, id)
}

/// What a library's contract says of its structs' fields, each fact by its
/// owner, sorted by key (ADR 0906): those `ref_cells` says are quiet (ADR
/// 0863, amended), and those whose fn pointers may only be Rust functions,
/// `pointers`' (ADR 0330, amended).
pub(super) fn fields(
    tcx: TyCtxt<'_>,
    ref_cells: Option<&super::drops::RefCells<'_>>,
    pointers: &super::analysis::FnPointers<'_>,
) -> Vec<crate::library::Field> {
    let mut facts: std::collections::BTreeMap<String, BTreeSet<crate::library::FieldFact>> = Default::default();
    for (id, fact) in ref_cells
        .into_iter()
        .flat_map(|found| found.fields())
        .chain(pointers.rust_fields())
    {
        facts.entry(key(tcx, id)).or_default().insert(fact);
    }
    (facts.into_iter())
        .map(|(key, facts)| crate::library::Field { key, facts })
        .collect()
}

/// What a library's contract calls each of `tys`, sorted, each once (ADR
/// 0863): as it calls a type an `Rc` it counts points at (ADR 0852).
pub(super) fn shapes<'tcx>(
    tcx: TyCtxt<'tcx>,
    tys: impl IntoIterator<Item = rustc_middle::ty::Ty<'tcx>>,
) -> Vec<RcShape> {
    let mut shapes: Vec<RcShape> = tys.into_iter().map(|ty| rc_shape(tcx, ty)).collect();
    shapes.sort();
    shapes.dedup();
    shapes
}

/// May `ty`, a `RefCell`'s type, be one `shape` names, a library's, as
/// `analysis::may_be_cell` says of two (ADR 0863): a type parameter of
/// either any type, a slice's what it's made of, and a `dyn`'s any type
/// where a value may be made one in place (ADR 0960).
pub(in crate::lower) fn may_be_cell_shape<'tcx>(
    tcx: TyCtxt<'tcx>,
    shape: &RcShape,
    ty: rustc_middle::ty::Ty<'tcx>,
) -> bool {
    may_be_cell_shape_at(tcx, (shape, ty), true)
}

/// `may_be_cell_shape` of `shape` and `ty`, where `unsized_here`, a `dyn`
/// there may be any type made one in place.
fn may_be_cell_shape_at<'tcx>(
    tcx: TyCtxt<'tcx>,
    (shape, ty): (&RcShape, rustc_middle::ty::Ty<'tcx>),
    unsized_here: bool,
) -> bool {
    use rustc_middle::ty;
    let dyn_shape = shape.of.as_deref().is_some_and(|of| of.starts_with("dyn "));
    match ty.kind() {
        ty::Dynamic(..) if unsized_here => true,
        _ if dyn_shape && unsized_here => true,
        ty::Slice(item) | ty::Array(item, _) => {
            shape.of.as_deref().is_none_or(|of| matches!(of, "[]" | "[;]"))
                && shape
                    .args
                    .first()
                    .is_none_or(|arg| may_be_cell_shape_at(tcx, (arg, *item), false))
        }
        _ => match (&shape.of, super::analysis::head(tcx, ty)) {
            (Some(of), Some((head, args))) => {
                let tail = super::analysis::unsized_in_place(tcx, ty);
                head_key(tcx, &head) == *of
                    && shape.args.len() == args.len()
                    && (shape.args.iter().zip(args).enumerate())
                        .all(|(i, (shape, arg))| may_be_cell_shape_at(tcx, (shape, arg), tail == Some(i)))
            }
            _ => true,
        },
    }
}

/// What a library's contract calls `head`, a type's constructor (ADR 0852).
fn head_key(tcx: TyCtxt<'_>, head: &super::analysis::Head) -> String {
    match head {
        super::analysis::Head::Def(kind, id) => format!("{kind}{}", key(tcx, *id)),
        super::analysis::Head::Named(name) => name.clone(),
    }
}

/// What a library's contract calls `ty`, a type an `Rc` it counts points
/// at, `_` where it may be any (ADR 0852).
fn rc_shape<'tcx>(tcx: TyCtxt<'tcx>, ty: rustc_middle::ty::Ty<'tcx>) -> RcShape {
    match super::analysis::head(tcx, ty) {
        Some((head, args)) => RcShape {
            of: Some(head_key(tcx, &head)),
            args: args.into_iter().map(|arg| rc_shape(tcx, arg)).collect(),
        },
        None => RcShape {
            of: None,
            args: Vec::new(),
        },
    }
}

/// May `ty` be the type `shape` names, a library's, a type parameter of
/// either any type (ADR 0852)? As `rc_counts::may_be` says of two types.
fn may_be_shape<'tcx>(tcx: TyCtxt<'tcx>, shape: &RcShape, ty: rustc_middle::ty::Ty<'tcx>) -> bool {
    match (&shape.of, super::analysis::head(tcx, ty)) {
        (Some(of), Some((head, args))) => {
            *of == head_key(tcx, &head)
                && shape.args.len() == args.len()
                && shape
                    .args
                    .iter()
                    .zip(args)
                    .all(|(shape, arg)| may_be_shape(tcx, shape, arg))
        }
        _ => true,
    }
}

/// What this crate's dependencies export, looked up by the `DefId`s rustc
/// gives their items here.
/// What the libraries can hand this crate, through the items of theirs
/// its bodies use: their parameters, results and fields (ADR 0626).
#[derive(Default)]
pub(super) struct Reached<'tcx> {
    /// A function a library made, its `impl Fn`, the one way its closure
    /// reaches the crate, which may hold a value with a destructor where the
    /// crate makes it a `dyn Fn`.
    pub(super) functions: bool,
    /// The libraries with a `dyn Fn` in what they export, which may hold
    /// one where theirs drop, and is given the crate's (ADR 0626, amended).
    pub(super) fn_dyns_of: std::collections::BTreeSet<String>,
    /// What each `Rc` or `Weak` a library's items hold points at, of
    /// their signatures and fields, not of a type parameter a consumer
    /// gives, which their code reads by a dictionary, as the crate has it:
    /// counted where the library counts it, which the crate then does,
    /// whose count they share (ADRs 0523, 0852).
    pub(super) counted_rcs: Vec<rustc_middle::ty::Ty<'tcx>>,
    /// And where it doesn't, each shared with the crate as the value
    /// itself, of the library named (ADRs 0844, 0852).
    pub(super) uncounted_rcs: Vec<(rustc_middle::ty::Ty<'tcx>, String)>,
}

pub(super) struct Foreign<'a, 'tcx> {
    tcx: TyCtxt<'tcx>,
    dependencies: &'a Dependencies,
    /// The libraries' functions the crate compiles itself (ADR 0914), which
    /// are its own, no item of theirs it imports.
    instances: &'a super::instances::Instances,
    /// Each library item a module names through its library's root
    /// (`rooted_paths`), found where first asked.
    rooted: std::cell::OnceCell<HashSet<(LocalModId, DefId)>>,
}

impl<'a, 'tcx> Foreign<'a, 'tcx> {
    pub(super) fn new(
        tcx: TyCtxt<'tcx>,
        dependencies: &'a Dependencies,
        instances: &'a super::instances::Instances,
    ) -> Self {
        Foreign {
            tcx,
            dependencies,
            instances,
            rooted: std::cell::OnceCell::new(),
        }
    }

    /// The crate's type context, which its queries are asked in.
    pub(super) fn tcx(&self) -> TyCtxt<'tcx> {
        self.tcx
    }

    /// Whether `id`, a library's function or a closure of one, is compiled
    /// in this crate (ADR 0914).
    pub(super) fn is_instance(&self, id: DefId) -> bool {
        self.instances.contains(self.tcx, id)
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
                && !library.builtin
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

    /// Is `id` a dependency's, one rust-js compiled as a library? Of
    /// `@rust-js/std`, built into the compiler, only what it compiled: the
    /// rest of std is std's (ADR 0561).
    pub(super) fn in_library(&self, id: DefId) -> bool {
        !id.is_local()
            && !super::recognition::not_compiled_for_build(self.tcx, id)
            && (self.dependencies.libraries.get(self.tcx.crate_name(id.krate).as_str())).is_some_and(|library| {
                !library.builtin || {
                    let key = key(self.tcx, id);
                    library.items.contains_key(&key) || library.impls.contains(&key)
                }
            })
    }

    pub(super) fn type_facts(&self, id: DefId) -> Vec<(u32, super::recognition::TypeFact)> {
        (self.item(id).into_iter().flat_map(|item| &item.calling.params))
            .flat_map(|param| param.facts.iter().map(move |word| (param.index, word)))
            .filter_map(|(index, word)| super::recognition::TypeFact::of_word(word).map(|fact| (index, fact)))
            .collect()
    }

    /// What `module` imports library item `id` as, from where: by its
    /// package's name, as its root re-exports it, where the module's Rust
    /// names it through the library's root; else by its module's file (the
    /// user's decision, ADR 1008, amended).
    pub(super) fn import_key(&self, id: DefId, module: LocalModId) -> Option<(String, String)> {
        let item = self.item(id)?;
        let rooted = || {
            self.rooted
                .get_or_init(|| rooted_paths(self.tcx))
                .contains(&(module, id))
        };
        Some(match &item.rooted {
            Some(by_package) if rooted() => by_package.clone(),
            _ => (item.from.clone(), item.export.clone()),
        })
    }

    pub(super) fn item(&self, id: DefId) -> Option<&'a Imported> {
        // An instance is the crate's own (ADR 0914); a std body this build
        // doesn't compile, a `MEANING_ONLY` adapter's, no library's (ADR
        // 0932).
        if id.is_local() || self.is_instance(id) || super::recognition::not_compiled_for_build(self.tcx, id) {
            return None;
        }
        let library = self
            .dependencies
            .libraries
            .get(self.tcx.crate_name(id.krate).as_str())?;
        library.items.get(&key(self.tcx, id))
    }

    /// How a library's `module` holds `binding` by its name `ident` there,
    /// if it does (ADR 0633).
    pub(super) fn held(&self, module: DefId, ident: rustc_span::Symbol, binding: DefId) -> Option<&'a HeldImport> {
        let library = (self.dependencies.libraries).get(self.tcx.crate_name(module.krate).as_str())?;
        let held = library.held.get(&(key(self.tcx, module), ident.to_string()))?;
        (held.binding == key(self.tcx, binding)).then_some(held)
    }

    /// Every binding the libraries' modules hold, as the imports they'd be.
    pub(super) fn all_held(&self) -> impl Iterator<Item = &'a HeldImport> {
        (self.dependencies.libraries.values()).flat_map(|library| library.held.values())
    }

    /// Is `id` a library's that its JS doesn't export, which this crate
    /// compiles itself where it uses it, from the MIR its library's metadata
    /// keeps (ADR 0914, amended): a trait impl of its, whose dictionary its
    /// program makes only where its own code asks for it, and its methods;
    /// or a trait's default, which a consumer's impl of the trait calls. Not
    /// the method of an impl whose dictionary it exports, called through it
    /// (`through_dictionary`); nor a derive's that rust-js writes itself,
    /// `==` of a struct, nor a binding's, JS's; nor a function or an
    /// inherent method, which its program exports where it's public, as its
    /// manifest says (`unlisted`).
    pub(super) fn compiles(&self, id: DefId) -> bool {
        use rustc_hir::def::DefKind;
        let tcx = self.tcx;
        let own = |imp: DefId| self.has_impl(imp) && self.item(imp).is_none();
        !id.is_local()
            && self.is_library(id.krate)
            && !self.is_instance(id)
            && self.item(id).is_none()
            && match tcx.def_kind(id) {
                DefKind::Impl { of_trait: true } => own(id),
                DefKind::AssocFn if tcx.is_mir_available(id) && !super::bindings::is_binding(tcx, id) => {
                    match tcx.trait_impl_of_assoc(id) {
                        Some(imp) => own(imp),
                        None => tcx.trait_of_assoc(id).is_some(),
                    }
                }
                _ => false,
            }
    }

    /// The impl of `id`, a library's impl's method its JS doesn't export by
    /// name, whose dictionary it does: what a call of it is a call through,
    /// `strToKebabCase.toKebabCase(title)` (ADR 0914, amended).
    pub(super) fn through_dictionary(&self, id: DefId) -> Option<DefId> {
        let imp = self.tcx.trait_impl_of_assoc(id)?;
        (!id.is_local() && !self.is_instance(id) && self.item(id).is_none() && self.item(imp).is_some()).then_some(imp)
    }

    /// Why `id` can't be used, if it's a library's its manifest doesn't list
    /// and the crate can't compile itself (`compiles`).
    pub(super) fn unlisted(&self, id: DefId) -> Option<String> {
        (self.in_library(id) && self.item(id).is_none() && !self.is_instance(id) && !self.compiles(id)).then(|| {
            format!(
                "rust-js: `{}` isn't one of the items `{}`'s manifest exports",
                self.tcx.def_path_str(id),
                self.tcx.crate_name(id.krate)
            )
        })
    }

    /// May `id`, a library's, return `Err(fmt::Error)` (ADR 0187)?
    /// Of a method called through its impl's dictionary, its dictionary's
    /// (ADR 0914, amended).
    pub(super) fn fails(&self, id: DefId) -> bool {
        let item = self.item(id).or_else(|| self.item(self.through_dictionary(id)?));
        item.is_some_and(|item| item.calling.has(ItemFact::Fails))
    }

    /// Does a library's `id` step its type parameter `index` as it is, by
    /// `by`, given its dictionary (ADRs 0411, 0548)?
    pub(super) fn steps(&self, id: DefId, index: u32, by: DefId) -> bool {
        let by = key(self.tcx, by);
        self.item(id)
            .and_then(|item| item.calling.param(index))
            .is_some_and(|param| param.stepped.contains(&by))
    }

    /// Does a library's `id` take its caller's location (ADR 0427)?
    pub(super) fn takes_location(&self, id: DefId) -> bool {
        self.item(id)
            .is_some_and(|item| item.calling.has(ItemFact::TakesLocation))
    }

    /// Does a library's `id` take JS's rest arguments, its last parameter
    /// `#[rust_js::rest]` (ADR 0664)?
    pub(super) fn takes_rest(&self, id: DefId) -> bool {
        self.item(id).is_some_and(|item| item.calling.has(ItemFact::Rest))
    }

    /// Does any of the libraries make a `dyn Any` of what drops, which the
    /// crate may be handed, or catch as what unwound (ADR 0691, amended)?
    pub(super) fn makes_dropping_anys(&self) -> bool {
        (self.dependencies.libraries.values()).any(|library| !library.builtin && library.makes_dropping_anys)
    }

    /// Do the `dyn Fn`s of `name`, a library, drop: does it make one of
    /// what drops, or is handed one so, which a `dyn Fn` it hands the crate
    /// may then be, and which drops one it's given (ADR 0626, amended)?
    pub(super) fn drops_fn_dyns(&self, name: &str) -> bool {
        (self.dependencies.libraries.get(name))
            .is_some_and(|library| !library.builtin && library.makes_dropping_fn_dyns)
    }

    /// Does any of the libraries keep a `Vec`'s capacity (ADR 0419)?
    pub(super) fn keeps_capacity(&self) -> bool {
        self.dependencies
            .libraries
            .values()
            .any(|library| library.keeps_capacity)
    }

    /// Of `of`, a library's item, whose own types are `tys`, given `args`:
    /// what each `Rc` or `Weak` they hold points at, as the crate has it,
    /// counted where the library counts it (ADRs 0844, 0852).
    fn holds_rcs(
        &self,
        of: DefId,
        (tys, args): (Vec<rustc_middle::ty::Ty<'tcx>>, rustc_middle::ty::GenericArgsRef<'tcx>),
        reached: &mut Reached<'tcx>,
    ) {
        let tcx = self.tcx;
        for part in tys.iter().flat_map(|t| t.walk()).filter_map(|part| part.as_type()) {
            let Some(pointee) =
                (super::recognition::rc_pointee(tcx, part)).or_else(|| super::recognition::weak_pointee(tcx, part))
            else {
                continue;
            };
            let pointee = rustc_middle::ty::EarlyBinder::bind(tcx, pointee).instantiate(tcx, args);
            let unnormalized = tcx.erase_and_anonymize_regions(pointee.skip_normalization());
            let env = rustc_middle::ty::TypingEnv::fully_monomorphized();
            let pointee = tcx.try_normalize_erasing_regions(env, pointee).unwrap_or(unnormalized);
            match self.counts_rc(of.krate, pointee) {
                true => reached.counted_rcs.push(pointee),
                false => reached
                    .uncounted_rcs
                    .push((pointee, tcx.crate_name(of.krate).to_string())),
            }
        }
    }

    /// Is `pointee` a type of a library's that counts no `Rc` that may be
    /// it, whose contract then decides it for every consumer of either,
    /// refusing one that would count it (ADR 0852, amended)?
    pub(super) fn counts_no_rc_of_its_type(&self, pointee: rustc_middle::ty::Ty<'tcx>) -> bool {
        match *pointee.kind() {
            rustc_middle::ty::Adt(adt, _) => {
                let krate = adt.did().krate;
                !adt.did().is_local() && self.is_library(krate) && !self.counts_rc(krate, pointee)
            }
            _ => false,
        }
    }

    /// Does `krate`, a library, count an `Rc` of `pointee` (ADR 0852)?
    fn counts_rc(&self, krate: rustc_span::def_id::CrateNum, pointee: rustc_middle::ty::Ty<'tcx>) -> bool {
        (self.dependencies.libraries.get(self.tcx.crate_name(krate).as_str()))
            .is_some_and(|library| (library.counted_rcs.iter()).any(|shape| may_be_shape(self.tcx, shape, pointee)))
    }

    /// What each library does with its `RefCell`s (ADR 0863), by its name.
    /// Each field of a library's struct its contract says is `fact`, by its
    /// key, and the library that says so (ADR 0906).
    /// Whether `field`, a library's struct's, is one its contract says
    /// `fact` of (ADR 0906).
    pub(super) fn field_has(&self, field: DefId, fact: crate::library::FieldFact) -> bool {
        !field.is_local()
            && (self
                .dependencies
                .libraries
                .get(self.tcx.crate_name(field.krate).as_str()))
            .and_then(|library| library.fields.get(&key(self.tcx, field)))
            .is_some_and(|facts| facts.contains(&fact))
    }

    pub(super) fn fields_saying(&self, fact: crate::library::FieldFact) -> HashMap<String, String> {
        (self.dependencies.libraries.iter())
            .flat_map(|(name, library)| {
                (library.fields.iter())
                    .filter(move |(_, facts)| facts.contains(&fact))
                    .map(move |(key, _)| (key.clone(), name.clone()))
            })
            .collect()
    }

    pub(super) fn ref_cell_facts(&self) -> Vec<(String, crate::library::RefCellFacts)> {
        let mut facts: Vec<_> = (self.dependencies.libraries.iter())
            .filter(|(_, library)| !library.ref_cells.is_empty())
            .map(|(name, library)| (name.clone(), library.ref_cells.clone()))
            .collect();
        facts.sort_by(|a, b| a.0.cmp(&b.0));
        facts
    }

    /// Do the writers of `krate`, a library's, take their `Formatter`'s
    /// options (ADR 0137)?
    pub(super) fn writers_take_options(&self, krate: rustc_span::def_id::CrateNum) -> bool {
        (self.dependencies.libraries.get(self.tcx.crate_name(krate).as_str()))
            .is_some_and(|library| library.writers_take_options)
    }

    /// May any of the libraries' items?
    pub(super) fn any_fails(&self) -> bool {
        self.all().any(|item| item.calling.has(ItemFact::Fails))
    }

    /// Is `id` a library's trait impl, one its consumers call?
    pub(super) fn has_impl(&self, id: DefId) -> bool {
        !id.is_local()
            && !super::recognition::not_compiled_for_build(self.tcx, id)
            && self
                .dependencies
                .libraries
                .get(self.tcx.crate_name(id.krate).as_str())
                .is_some_and(|library| library.impls.contains(&key(self.tcx, id)))
    }

    /// What `id`, a library's type, is declared as: its module's `.d.ts`,
    /// by its JS file's specifier, and its name there, where its library's
    /// declarations export it (ADR 0210, amended).
    pub(super) fn declared_type(&self, id: DefId) -> Option<&'a (String, String, bool)> {
        if id.is_local() {
            return None;
        }
        let library = (self.dependencies.libraries).get(self.tcx.crate_name(id.krate).as_str())?;
        library.types.get(&key(self.tcx, id))
    }

    /// Is `id`, a library's type, equal only to itself, as its contract
    /// says (ADR 0864)?
    pub(super) fn equal_only_to_itself(&self, id: DefId) -> bool {
        !id.is_local()
            && (self.dependencies.libraries.get(self.tcx.crate_name(id.krate).as_str()))
                .is_some_and(|library| library.identities.contains(&key(self.tcx, id)))
    }

    /// Is each call of `id`, a library's function, what it's given, as its
    /// contract says (ADR 0584, amended)?
    pub(super) fn gives_its_argument(&self, id: DefId) -> bool {
        !id.is_local()
            && (self.dependencies.libraries.get(self.tcx.crate_name(id.krate).as_str()))
                .is_some_and(|library| library.gives_argument.contains(&key(self.tcx, id)))
    }

    /// Is `krate` a library's, one rust-js built for this crate to use, not
    /// `@rust-js/std` (ADR 0100)?
    pub(super) fn is_library(&self, krate: rustc_span::def_id::CrateNum) -> bool {
        (self.dependencies.libraries.get(self.tcx.crate_name(krate).as_str())).is_some_and(|library| !library.builtin)
    }

    /// What the libraries can hand this crate (ADR 0626): of each type its
    /// bodies' values have, what a library's part of it reaches, a
    /// library's function's signature, a library's type's fields. Not what
    /// the crate's own types hold, which it sees.
    pub(super) fn reached(&self, all_bodies: &[&super::Body<'tcx>]) -> Reached<'tcx> {
        let mut reached = Reached::default();
        if !self.any() {
            return reached;
        }
        let mut seen = HashSet::new();
        for body in all_bodies {
            for expr in body.thir.exprs.iter() {
                for part in expr.ty.walk().filter_map(|part| part.as_type()) {
                    self.reach(part, None, &mut seen, &mut reached);
                }
            }
        }
        reached
    }

    /// What `ty` reaches of a library's, `exported` of the library whose
    /// export it's in.
    fn reach(
        &self,
        ty: rustc_middle::ty::Ty<'tcx>,
        exported: Option<rustc_span::def_id::CrateNum>,
        seen: &mut HashSet<(rustc_middle::ty::Ty<'tcx>, Option<rustc_span::def_id::CrateNum>)>,
        reached: &mut Reached<'tcx>,
    ) {
        use rustc_middle::ty;
        if !seen.insert((ty, exported)) {
            return;
        }
        let tcx = self.tcx;
        let library = |id: DefId| self.is_library(id.krate);
        // Not what a closure of the crate's holds, which a library's generic
        // function is given, `then(move |_| ..)`'s: the crate made it of
        // what it sees, nothing the library hands it.
        let mut parts = |of: DefId, tys: Vec<ty::Ty<'tcx>>, reached: &mut Reached<'tcx>| {
            for t in tys {
                let mut walk = t.walk();
                while let Some(part) = walk.next() {
                    let Some(part) = part.as_type() else { continue };
                    if let ty::Closure(id, _) | ty::CoroutineClosure(id, _) | ty::Coroutine(id, _) = *part.kind()
                        && id.is_local()
                    {
                        walk.skip_current_subtree();
                        continue;
                    }
                    self.reach(part, Some(of.krate), seen, reached);
                }
            }
        };
        match *ty.kind() {
            ty::Alias(
                _,
                ty::AliasTy {
                    kind: ty::Opaque { def_id },
                    ..
                },
            ) => reached.functions |= library(def_id),
            ty::Dynamic(predicates, ..)
                if let Some(krate) = exported
                    && super::recognition::is_fn_dyn(tcx, predicates) =>
            {
                reached.fn_dyns_of.insert(tcx.crate_name(krate).to_string());
            }
            ty::FnDef(..)
                if let Some((id, args)) = super::fn_def(ty)
                    && library(id) =>
            {
                let declared = tcx
                    .instantiate_bound_regions_with_erased(tcx.fn_sig(id).instantiate_identity().skip_normalization());
                self.holds_rcs(id, (declared.inputs_and_output.to_vec(), args), reached);
                let sig = tcx.fn_sig(id).instantiate(tcx, args).skip_normalization().skip_binder();
                parts(id, sig.inputs_and_output.to_vec(), reached);
            }
            ty::Adt(adt, args) if library(adt.did()) => {
                let declared = adt
                    .all_fields()
                    .map(|field| tcx.type_of(field.did).instantiate_identity().skip_normalization());
                self.holds_rcs(adt.did(), (declared.collect(), args), reached);
                let fields = adt.all_fields().map(|field| field.ty(tcx, args).skip_normalization());
                parts(adt.did(), fields.collect(), reached);
            }
            _ => {}
        }
    }

    /// Does this crate use a library?
    /// Is any library used, one whose values a program's may be, so their
    /// representations are shared (ADRs 0100, 0523)? Not `@rust-js/std`,
    /// built into the compiler, whose bodies are std's own over the
    /// program's values as the program has them (ADR 0561).
    pub(super) fn any(&self) -> bool {
        self.dependencies.libraries.values().any(|library| !library.builtin)
    }

    /// Every item the dependencies export, as the imports they'd be.
    pub(super) fn all(&self) -> impl Iterator<Item = &'a Imported> {
        self.dependencies
            .libraries
            .values()
            .flat_map(|library| library.items.values())
    }
}

/// Each library item a module of the crate names through its library's
/// root, by the module: `use react_isomorphic::CHILDREN;` of a `pub use
/// react_client::CHILDREN;` there, a path whose last but one segment is the
/// library's root, its `use`s' and its expressions' (the user's decision,
/// ADR 1008, amended).
fn rooted_paths(tcx: TyCtxt<'_>) -> HashSet<(LocalModId, DefId)> {
    use rustc_hir::def::{DefKind, Res};
    use rustc_hir::intravisit::{self, Visitor};
    struct Paths<'tcx> {
        tcx: TyCtxt<'tcx>,
        rooted: HashSet<(LocalModId, DefId)>,
    }
    impl<'tcx> Visitor<'tcx> for Paths<'tcx> {
        type NestedFilter = rustc_middle::hir::nested_filter::All;
        fn maybe_tcx(&mut self) -> TyCtxt<'tcx> {
            self.tcx
        }
        fn visit_path(&mut self, path: &rustc_hir::Path<'tcx>, id: rustc_hir::HirId) {
            if let Res::Def(_, item) = path.res
                && !item.is_local()
                && let [.., parent, _] = path.segments
                && let Res::Def(DefKind::Mod, module) = parent.res
                && module == item.krate.as_def_id()
            {
                self.rooted.insert((self.tcx.parent_module(id), item));
            }
            intravisit::walk_path(self, path);
        }
    }
    let mut paths = Paths {
        tcx,
        rooted: HashSet::new(),
    };
    tcx.hir_visit_all_item_likes_in_crate(&mut paths);
    paths.rooted
}
