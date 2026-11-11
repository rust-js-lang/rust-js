//! Translate checked Rust identities to the library contract (ADR 0100): the
//! items a library exports, and the ones a consumer imports.

mod built_types;
mod cells;
mod compiled_here;
mod exports;
mod reached;
pub(super) use compiled_here::merging;
pub(super) use exports::exports;

use super::bindings::HeldBinding;
use super::{FnInfo, module_path};
use crate::collections::{IndexMap, IndexSet};
use crate::library::{Dependencies, Held, HeldImport, Imported, Item, ItemFact, Library, RcShape, Reexported, VERSION};
use rustc_middle::ty::TyCtxt;
use rustc_span::def_id::{DefId, LOCAL_CRATE, LocalModId};
use std::collections::BTreeSet;

/// What a library and its consumers both call an item: rustc's `DefPathHash`
/// of it, the same in every crate that sees it. Of std's own, `core`'s,
/// `alloc`'s and `std`'s, its crate's name and its def path, each part's
/// disambiguator too, `core::iter::adapters::inspect::{impl#3}::next`: the
/// same of the std `@rust-js/std` was compiled from, built to unwind, as
/// of the one the toolchain ships, built to abort, whose crates' ids
/// differ (#424).
fn key(tcx: TyCtxt<'_>, id: DefId) -> String {
    match super::recognition::is_std_crate(tcx, id) {
        true => format!(
            "{}{}",
            tcx.crate_name(id.krate),
            tcx.def_path(id).to_string_no_crate_verbose()
        ),
        false => tcx.def_path_hash(id).0.to_hex(),
    }
}

/// Whether the writers of a library one of whose bodies, `ids`, the crate
/// compiles itself (ADR 0914) take a `Formatter`'s options, as its
/// manifest says: a body compiled from another crate's MIR has no THIR to
/// say what it formats, `{:#?}` (#1325). Not `@rust-js/std`'s, which takes
/// every program's.
pub(super) fn instances_take_options(
    tcx: TyCtxt<'_>,
    dependencies: &Dependencies,
    ids: impl IntoIterator<Item = DefId>,
) -> bool {
    (ids.into_iter().filter(|id| !id.is_local())).any(|id| {
        dependencies
            .libraries
            .get(&library_key(tcx, id.krate))
            .is_some_and(|library| !library.builtin && library.writers_take_options)
    })
}

/// The type parameters `id`, a library's function, shows pretty or in
/// hex, by index, as its manifest says (#1331): none of another.
pub(super) fn shown_marks(
    tcx: TyCtxt<'_>,
    dependencies: &Dependencies,
    id: DefId,
) -> Vec<(u32, super::analysis::Shown)> {
    use crate::library::ParamFact;
    let Some(item) =
        (dependencies.libraries.get(&library_key(tcx, id.krate))).and_then(|library| library.items.get(&key(tcx, id)))
    else {
        return Vec::new();
    };
    (item.calling.params.iter())
        .map(|param| {
            let shown = super::analysis::Shown {
                pretty: param.has(ParamFact::ShownPretty),
                hex: param.has(ParamFact::ShownHex),
            };
            (param.index, shown)
        })
        .filter(|(_, shown)| shown.any())
        .collect()
}

/// The key of the library `krate`'s items are of, by which `Dependencies`
/// knows it: `@rust-js/std`'s name, which no crate has, of std's crates,
/// not their names, which a program's crate may have, a workspace's `core`
/// (ADR 1054); any other's its crate's name and identity
/// (`Library::key`), never its name alone, which std's own `cfg_if` and
/// crates.io's share (#1130).
fn library_key(tcx: TyCtxt<'_>, krate: rustc_span::def_id::CrateNum) -> String {
    match super::recognition::std_crate(tcx, krate) {
        Some(_) => crate::runtime::PACKAGE.to_string(),
        None => crate::library::key(tcx.crate_name(krate).as_str(), &crate_id(tcx, krate)),
    }
}

/// rustc's identity of `krate`, its `StableCrateId`, as a library's
/// manifest writes it (`Library::crate_id`).
fn crate_id(tcx: TyCtxt<'_>, krate: rustc_span::def_id::CrateNum) -> String {
    format!("{:016x}", tcx.stable_crate_id(krate).as_u64())
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
    pub(super) drop_params: &'a IndexMap<DefId, Vec<u32>>,
    /// The type parameters each function shows pretty or in hex (#1331).
    pub(super) shown_params: &'a IndexMap<DefId, IndexMap<u32, super::analysis::Shown>>,
    /// The type parameters whose `Copy` bound takes a copy function (ADRs
    /// 0289, 0914), whose `Clone` bound takes its dictionary (ADR 0914,
    /// amended), and which are stepped by a trait (ADR 0548).
    pub(super) given: super::traits::Given<'a>,
    pub(super) failing: &'a IndexSet<DefId>,
    /// Whether any write of its may fail, of a function whatever it
    /// returns, a writer's `write` too (#1072).
    pub(super) writes_may_fail: bool,
    pub(super) no_drops: &'a IndexMap<DefId, BTreeSet<u32>>,
    pub(super) type_facts: &'a IndexMap<DefId, Vec<(u32, super::recognition::TypeFact)>>,
    /// What its consumers' impls of its traits carry of a std supertrait's
    /// associated types (#990).
    pub(super) carried_super: &'a super::analysis::type_facts::CarriedSuper,
    /// The types built of each one's slots it asks the layout of, which
    /// its manifest says by a template of them (#737).
    pub(super) built_slots: (&'a Foreign<'a, 'tcx>, &'a super::traits::BuiltSlots<'tcx>),
    /// Those that take their caller's location (ADR 0427).
    pub(super) caller_locations: &'a IndexSet<DefId>,
    /// Those that read `js::arguments()` (rd332), and the parameters'
    /// tuples of its `function::with_this` functions that do (#553).
    pub(super) reads_arguments: &'a super::analysis::arguments::ReadsArguments<'tcx>,
    /// Whether it keeps a `Vec`'s capacity, which its consumers then do
    /// too, as what they give it may be asked (ADR 0419).
    pub(super) keeps_capacity: bool,
    /// Whether its writers take their `Formatter`'s options (ADR 0137).
    pub(super) writers_take_options: bool,
    /// Whether an integer's `{:?}` its writers write may be hex (ADR 0559).
    pub(super) debug_hex: bool,
    /// Its functions that read a `Formatter`'s options (#1026).
    pub(super) reads_formatter_options: &'a IndexSet<DefId>,
    /// Its functions whose JS may merge what Rust keeps apart, each that
    /// calls one among them (`merging`, #1052).
    pub(super) merges_identities: &'a IndexSet<DefId>,
    /// Whether it makes a `dyn Any` of what drops (ADR 0691, amended).
    pub(super) makes_dropping_anys: bool,
    /// Whether it makes a `dyn Fn` of what drops (ADR 0626, amended).
    pub(super) makes_dropping_fn_dyns: bool,
    /// A program's, `--tested`'s (#597): the trait constants its
    /// dictionaries carry, its generic code's, a library's every one; and its
    /// own traits whose `dyn`s drop nothing, none of a library's.
    pub(super) program: Option<(&'a IndexSet<DefId>, Vec<DefId>)>,
    /// What the `Rc`s it counts point at (ADR 0852).
    pub(super) counted: &'a super::analysis::Counted<'tcx>,
    /// What it changes in place, which its JS copies (ADR 0914, amended).
    pub(super) changed: super::copies::Changed<'a, 'tcx>,
    /// What it does with its `RefCell`s (ADR 0863).
    pub(super) ref_cells: crate::library::RefCellFacts,
    /// What it says of its structs' fields (ADRs 0863, amended, 0906).
    pub(super) fields: Vec<crate::library::Field>,
    /// The thread-locals made where they're first used (ADR 0037, amended).
    pub(super) lazy_locals: &'a IndexSet<rustc_span::def_id::LocalDefId>,
    /// The exported thread-locals and `static mut`s that are their module's
    /// variables (ADRs 0270, 0096, amended).
    pub(super) variables: &'a IndexSet<DefId>,
    /// Its types equal only to themselves (ADR 0864).
    pub(super) identities: &'a [DefId],
    /// Its enums told apart by their keys (`bindings::flattened`, ADR 1071).
    pub(super) keyed: &'a IndexSet<DefId>,
    /// Its types whose `==` is their address's, whatever their `Hash` (ADR
    /// 0864, amended).
    pub(super) eq_by_address: &'a [DefId],
    /// Its functions each call of which is what it's given (ADR 0584,
    /// amended).
    pub(super) gives_argument: &'a [DefId],
    /// Its untagged enums whose `Display` writes each variant's value as
    /// it is (#557, rd730).
    pub(super) displays_value: &'a [DefId],
    /// The structs its code makes values of (ADR 1022, amended).
    pub(super) built: Vec<DefId>,
    /// The closures it clones, or gives to clone, and whether each is
    /// cloned apart (ADR 0522, amended; #1202).
    pub(super) cloned_closures: Vec<(DefId, bool)>,
    /// What each function's return alone owns of what it's given (ADR
    /// 0317, amended).
    pub(super) owned_by_return: &'a super::analysis::owned_by_return::OwnedByReturn,
    /// The own nullable parameters each call gives `null` or a value
    /// (`analysis/nulled_params.rs`, ADR 0861, amended).
    pub(super) nulled_params: &'a IndexSet<(DefId, usize)>,
}

/// What `id` asks of each of its type parameters that it asks anything of,
/// by index, as its contract says them: a `dropT`, no destructor, type
/// facts, and the traits it steps one by.
fn type_params<'tcx>(tcx: TyCtxt<'tcx>, id: DefId, contracts: &Contracts<'_, 'tcx>) -> Vec<crate::library::TypeParam> {
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
    for (&index, &shown) in contracts.shown_params.get(&id).into_iter().flatten() {
        let marks = &mut param(&mut params, index).marks;
        if shown.pretty {
            marks.insert(ParamFact::ShownPretty);
        }
        if shown.hex {
            marks.insert(ParamFact::ShownHex);
        }
    }
    for &(index, fact) in contracts.type_facts.get(&id).into_iter().flatten() {
        param(&mut params, index).facts.push(fact.word().to_string());
    }
    for &(_, index, by) in contracts.given.stepped.iter().filter(|&&(of, _, _)| of == id) {
        param(&mut params, index).stepped.push(key(tcx, by));
    }
    // Each built slot's type, of its others, which its consumers make of
    // what they give them (#737); refused where a template can't say it.
    let (foreign, built) = contracts.built_slots;
    let first = super::traits::item_slot(tcx, id, super::traits::drop_items(tcx, foreign, id).len());
    for (at, slot) in built.get(&id).into_iter().flatten().enumerate() {
        let Some(of) = built_types::template(tcx, foreign, id, slot.ty) else {
            super::refusal::refuse(
                tcx,
                tcx.def_span(id),
                super::refusal::Refused::Type("a library's function asking the layout of a type", slot.ty),
            );
            continue;
        };
        param(&mut params, first + at as u32).built = Some(crate::library::BuiltType {
            of,
            gates: slot.gates.clone(),
        });
    }
    let mut params: Vec<_> = params.into_values().collect();
    for param in &mut params {
        param.stepped.sort();
    }
    params
}

/// The keys of `ids`, sorted, as a contract lists them.
fn sorted_keys<'a>(tcx: TyCtxt<'_>, ids: impl Iterator<Item = &'a DefId>) -> Vec<String> {
    let mut keys: Vec<String> = ids.map(|&id| key(tcx, id)).collect();
    keys.sort();
    keys
}

/// Is `id` the crate's own, or std's compiled into `@rust-js/std` (ADR
/// 0561): not a library's it compiled itself, which is that library's (ADR
/// 0914).
fn own(tcx: TyCtxt<'_>, id: DefId) -> bool {
    id.is_local() || super::recognition::is_std_crate(tcx, id)
}

/// What a crate's JS exports of its own, each by its module's path and its
/// name there (`exported_names`).
pub(super) type Exported = IndexSet<(Vec<String>, String)>;

/// What each of `modules`' JS exports, by its module's path and its name
/// there: an item `export`ed, its default, those it exports by another
/// name, and those it re-exports, a namespace's method that's the function
/// it passes its parameters on to among them, `export { batchedUpdates }
/// from "./ReactFiberWorkLoop.js"` (rd292b).
/// A library's trait method without a default whose entries take its
/// caller's location, refused: its manifest, of functions, can't say so to
/// its consumers' calls through them (ADR 0427).
pub(super) fn refuse_unsaid_locations(
    tcx: TyCtxt<'_>,
    caller_locations: &IndexSet<DefId>,
    fns: &IndexMap<DefId, FnInfo>,
) {
    let mut unsaid: Vec<DefId> = (caller_locations.iter())
        .filter(|&&id| tcx.trait_of_assoc(id).is_some() && !fns.contains_key(&id))
        .filter(|&&id| reachable(tcx, id))
        .copied()
        .collect();
    unsaid.sort_by_key(|&id| tcx.def_span(id));
    for id in unsaid {
        super::refusal::refuse(
            tcx,
            tcx.def_span(id),
            super::refusal::Refused::Feature(
                "a library's trait method without a default that's given its caller's location through its dictionary",
            ),
        );
    }
}

pub(super) fn exported_names(modules: &[crate::program::UnlinkedModule]) -> Exported {
    let mut exported = IndexSet::default();
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
            .chain(
                module
                    .reexports
                    .iter()
                    .flat_map(|reexport| reexport.named.iter().map(|(_, alias)| alias)),
            )
        {
            export(name);
        }
    }
    exported
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
    /// one where theirs drop, and is given the crate's (ADR 0626, amended),
    /// by key (`library_key`).
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
    /// Each library item a module names through its library's root, and
    /// through another of its modules (`library_paths`), found where first
    /// asked.
    paths: std::cell::OnceCell<LibraryPaths>,
    /// What the crate's own writers take beyond their arguments.
    writes: super::analysis::Writes,
    /// Whether the crate reads a function's identity (`merges_identities`).
    reads_fn_identity: bool,
    /// Whether the crate reads a `Vec`'s capacity (`keeps_more`).
    asks_capacity: bool,
    /// Whether a copy of each library function asked can reach all it
    /// reads and calls (`instances::copy_reaches`), found once.
    copies_reach: std::cell::RefCell<IndexMap<DefId, bool>>,
}

impl<'a, 'tcx> Foreign<'a, 'tcx> {
    pub(super) fn new(
        tcx: TyCtxt<'tcx>,
        dependencies: &'a Dependencies,
        instances: &'a super::instances::Instances,
        (writes, reads_fn_identity): (super::analysis::Writes, bool),
    ) -> Self {
        Foreign {
            tcx,
            dependencies,
            instances,
            paths: std::cell::OnceCell::new(),
            writes,
            reads_fn_identity,
            asks_capacity: false,
            copies_reach: Default::default(),
        }
    }

    /// Itself, of a crate that reads a `Vec`'s capacity, `asks_capacity`
    /// (`keeps_more`, #1180's (5)).
    pub(super) fn asking_capacity(self, asks_capacity: bool) -> Self {
        Foreign { asks_capacity, ..self }
    }

    /// The library `krate`'s items are of, if it's one: `@rust-js/std` of
    /// std's crates, whatever their names, any other by its crate's name and
    /// identity (`library_key`).
    fn library(&self, krate: rustc_span::def_id::CrateNum) -> Option<&'a crate::library::Exports> {
        self.dependencies.libraries.get(&library_key(self.tcx, krate))
    }

    /// What the library JS file `from` exports but `default`, by the
    /// specifier the crate imports it by, as its library's manifest says
    /// (#629): of none of a library's, none.
    pub(super) fn file_exports(&self, from: &str) -> Option<&'a std::collections::BTreeSet<String>> {
        (self.dependencies.libraries.values()).find_map(|library| library.files.get(from))
    }

    /// Does the library `krate` change a value of `ty` in place, so that
    /// its JS copies one where Rust does (ADR 0914, amended)? Not one whose
    /// contract says none, or of a compiler that wrote no such fact.
    pub(super) fn changes(&self, krate: rustc_span::def_id::CrateNum, ty: rustc_middle::ty::Ty<'tcx>) -> bool {
        (self.library(krate))
            .is_some_and(|library| (library.changed.iter()).any(|shape| may_be_shape(self.tcx, shape, ty)))
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
    /// against. Asked of each library, of the crates rustc loaded of its
    /// name: one is the library, its identity and hash; another of its name,
    /// std's own `cfg_if` of rustc's sysroot, is another crate (#1130).
    pub(super) fn check(&self) -> bool {
        let mut matched = true;
        for library in self.dependencies.libraries.values() {
            if library.builtin {
                continue;
            }
            // Its hash is of its identity too, `StableCrateId`.
            let named: Vec<_> = (self.tcx.crates(()).iter())
                .filter(|&&krate| self.tcx.crate_name(krate).as_str() == library.name)
                .collect();
            let loaded = (named.iter()).any(|&&krate| self.tcx.crate_hash(krate).to_string() == library.crate_hash);
            if !named.is_empty() && !loaded {
                self.tcx.dcx().err(format!(
                    "rust-js: the metadata rustc loaded for `{}` isn't of the build its JS was made from: rebuild `{}`, and use the metadata rust-js writes beside its JS",
                    library.name, library.name
                ));
                matched = false;
            }
        }
        matched
    }

    /// Is `id` an item rust-js compiled, the crate's own or a library's
    /// (ADR 0100), whose JS and dictionaries are rust-js's, whichever crate
    /// is compiled now: the one answer of which items are, never
    /// `id.is_local()` alone, which a library's own compile and its
    /// consumer's answer apart. Any other is std's.
    pub(in crate::lower) fn compiled_by_rust_js(&self, id: DefId) -> bool {
        id.is_local() || self.in_library(id)
    }

    /// Is `id` a dependency's, one rust-js compiled as a library? Of
    /// `@rust-js/std`, built into the compiler, only what it compiled: the
    /// rest of std is std's (ADR 0561).
    pub(super) fn in_library(&self, id: DefId) -> bool {
        !id.is_local()
            && !super::recognition::not_compiled_for_build(self.tcx, id)
            && self.library(id.krate).is_some_and(|library| {
                !library.builtin || {
                    let key = key(self.tcx, id);
                    library.items.contains_key(&key) || library.impls.contains(&key)
                }
            })
    }

    /// What a library's trait's dictionaries carry of its associated type
    /// `assoc`, `Size` of smallvec's `Array::Item`, as its manifest says:
    /// what the crate's dictionaries of its own impls of the trait carry
    /// too, and what its code may read of any (ADR 0145, amended, #476).
    pub(super) fn carried(&self, assoc: DefId) -> Vec<super::recognition::TypeFact> {
        (self.library(assoc.krate).filter(|library| !library.builtin))
            .and_then(|library| library.associated.get(&key(self.tcx, assoc)))
            .into_iter()
            .flatten()
            .filter_map(|word| super::recognition::TypeFact::of_word(word))
            .collect()
    }

    /// What the dictionaries of the crate's impls of `trait_id`, a library's
    /// trait, carry of a std supertrait's associated types of their `Self`,
    /// as its manifest says, `IteratorItemSize` of `<Self as Iterator>::Item`
    /// (#990): what its defaults compiled over `Self` read.
    pub(in crate::lower) fn carried_super(&self, trait_id: DefId) -> Vec<(DefId, super::recognition::TypeFact)> {
        let Some(carried) = (self.library(trait_id.krate).filter(|library| !library.builtin))
            .and_then(|library| library.carried_super.get(&key(self.tcx, trait_id)))
        else {
            return Vec::new();
        };
        let supers = rustc_middle::ty::elaborate::supertrait_def_ids(self.tcx, trait_id);
        let assocs = supers.flat_map(|of| self.tcx.associated_item_def_ids(of).iter().copied());
        let mut facts = Vec::new();
        for assoc in assocs {
            for word in carried.get(&key(self.tcx, assoc)).into_iter().flatten() {
                facts.extend(super::recognition::TypeFact::of_word(word).map(|fact| (assoc, fact)));
            }
        }
        facts
    }

    /// A library's function's built slot `index`, as its manifest's
    /// template says it, in the function's terms (#737).
    pub(in crate::lower) fn built_slot(&self, id: DefId, index: u32) -> Option<super::traits::BuiltSlot<'tcx>> {
        let built = self.item(id)?.calling.param(index)?.built.as_ref()?;
        Some(super::traits::BuiltSlot {
            ty: built_types::made(self.tcx, self, id, &built.of)?,
            gates: built.gates.clone(),
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
    /// names it through the library's root (the user's decision, ADR 1008,
    /// amended); by the file of the library's module the Rust names it
    /// through first, where that module re-exports it, as its contract says
    /// (rd704, ADR 0874, amended); else by its module's file.
    pub(super) fn import_key(&self, id: DefId, module: LocalModId) -> Option<(String, String)> {
        let item = self.item(id)?;
        let paths = self.paths.get_or_init(|| library_paths(self.tcx));
        if let Some(by_package) = &item.rooted
            && paths.rooted.contains(&(module, id))
        {
            return Some(by_package.clone());
        }
        let through = (paths.through.get(&(module, id))).and_then(|&(holder, name)| {
            let library = self.library(holder.krate)?;
            let (reexported, imported) = library.reexported.get(&(key(self.tcx, holder), name.to_string()))?;
            (*reexported == key(self.tcx, id)).then(|| imported.clone())
        });
        Some(through.unwrap_or_else(|| (item.from.clone(), item.export.clone())))
    }

    pub(super) fn item(&self, id: DefId) -> Option<&'a Imported> {
        // An instance is the crate's own (ADR 0914); a std body this build
        // doesn't compile, a `MEANING_ONLY` adapter's, no library's (ADR
        // 0932).
        if id.is_local() || self.is_instance(id) || super::recognition::not_compiled_for_build(self.tcx, id) {
            return None;
        }
        let library = self.library(id.krate)?;
        library.items.get(&key(self.tcx, id))
    }

    /// How a library's `module` holds `binding` by its name `ident` there,
    /// if it does (ADR 0633).
    pub(super) fn held(&self, module: DefId, ident: rustc_span::Symbol, binding: DefId) -> Option<&'a HeldImport> {
        let library = self.library(module.krate)?;
        let held = library.held.get(&(key(self.tcx, module), ident.to_string()))?;
        (held.binding == key(self.tcx, binding)).then_some(held)
    }

    /// Every binding the libraries' modules hold, as the imports they'd be.
    pub(super) fn all_held(&self) -> impl Iterator<Item = &'a HeldImport> {
        (self.dependencies.libraries.values()).flat_map(|library| library.held.values())
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
    /// Of an impl's method, its dictionary's, the library's answer of the
    /// impl's code wherever it's compiled: called through the dictionary, or
    /// an instance the crate compiles itself (ADR 0914, amended; #1023).
    pub(super) fn fails(&self, id: DefId) -> bool {
        // As its manifest lists it, which `item` doesn't of an instance.
        let listed = |id: DefId| match id.is_local() || super::recognition::not_compiled_for_build(self.tcx, id) {
            true => None,
            false => self.library(id.krate)?.items.get(&key(self.tcx, id)),
        };
        let item = listed(id).or_else(|| listed(self.tcx.trait_impl_of_assoc(id)?));
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

    /// Does what `id`, a library's function, returns alone own its
    /// parameter `index`, as its contract says (ADR 0317, amended)?
    pub(super) fn owned_by_return(&self, id: DefId, index: u32) -> bool {
        self.item(id)
            .is_some_and(|item| item.calling.owned_by_return.contains(&index))
    }

    /// Does nothing `id`, a library's function, runs before it returns
    /// call its parameter `index`, which what it returns alone owns, as its
    /// contract says (ADR 0317, amended, rd621)?
    pub(super) fn quiet_by_return(&self, id: DefId, index: u32) -> bool {
        self.item(id)
            .is_some_and(|item| item.calling.quiet_by_return.contains(&index))
    }

    /// Does a library's `id` take its parameter `index` as `null` or a
    /// value only, as its contract says (ADR 0861, amended)?
    pub(super) fn nulled_param(&self, id: DefId, index: usize) -> bool {
        self.item(id)
            .is_some_and(|item| item.calling.nulled_params.contains(&(index as u32)))
    }

    /// Does a library's `id` take any parameter as `null` or a value only
    /// (ADR 0861, amended)?
    pub(super) fn has_nulled_params(&self, id: DefId) -> bool {
        self.item(id).is_some_and(|item| !item.calling.nulled_params.is_empty())
    }

    /// Does `id`, a library's function, read `js::arguments()`, as its
    /// contract says (rd332)?
    pub(super) fn reads_arguments(&self, id: DefId) -> bool {
        self.item(id).is_some_and(|item| item.calling.has(ItemFact::Arguments))
    }

    /// May a call of a `dyn Fn`, a generic one or a pointer, given `args`,
    /// its arguments' tuple, call a library's `function::with_this`
    /// function that reads `js::arguments()`, as its contract says (ADR
    /// 1048, amended, #553)?
    pub(super) fn may_call_reading_arguments(&self, args: rustc_middle::ty::Ty<'tcx>) -> bool {
        (self.dependencies.libraries.values()).any(|library| {
            !library.builtin && (library.with_this_arguments.iter()).any(|shape| may_be_shape(self.tcx, shape, args))
        })
    }

    /// Does `id`, a library's function, read more of its call than its
    /// parameters, `js::arguments()` or `js::this()`, as its contract says,
    /// so no closure that forwards to it is it (rd332, rd683)?
    pub(super) fn reads_its_call(&self, id: DefId) -> bool {
        self.reads_arguments(id) || self.reads_this(id)
    }

    /// Does `id`, a library's function, read `js::this()`, as its contract
    /// says (rd683)?
    pub(super) fn reads_this(&self, id: DefId) -> bool {
        self.item(id).is_some_and(|item| item.calling.has(ItemFact::This))
    }

    /// Does any of the libraries make a `dyn Any` of what drops, which the
    /// crate may be handed, or catch as what unwound (ADR 0691, amended)?
    pub(super) fn makes_dropping_anys(&self) -> bool {
        (self.dependencies.libraries.values()).any(|library| !library.builtin && library.makes_dropping_anys)
    }

    /// Do the `dyn Fn`s of the library `key` drop: does it make one of
    /// what drops, or is handed one so, which a `dyn Fn` it hands the crate
    /// may then be, and which drops one it's given (ADR 0626, amended)?
    pub(super) fn drops_fn_dyns(&self, key: &str) -> bool {
        (self.dependencies.libraries.get(key)).is_some_and(|library| !library.builtin && library.makes_dropping_fn_dyns)
    }

    /// The crate's name of the library `key`, which what's said of it names.
    pub(super) fn library_named<'k>(&self, key: &'k str) -> &'k str
    where
        'a: 'k,
    {
        (self.dependencies.libraries.get(key)).map_or(key, |library| library.name.as_str())
    }

    /// Does any of the libraries keep a `Vec`'s capacity (ADR 0419), as
    /// what the crate gives it may be asked? Not `@rust-js/std`, which
    /// keeps what its own bodies grow for every program (#1339), and asks
    /// no consumer to.
    pub(super) fn keeps_capacity(&self) -> bool {
        (self.dependencies.libraries.values()).any(|library| !library.builtin && library.keeps_capacity)
    }

    /// Of `of`, a library's item, whose own types are `tys`, given `args`:
    /// what each `Rc` or `Weak` they hold points at, as the crate has it,
    /// counted where the library counts it (ADRs 0844, 0852).
    pub(super) fn holds_rcs(
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
        self.library(krate)
            .is_some_and(|library| (library.counted_rcs.iter()).any(|shape| may_be_shape(self.tcx, shape, pointee)))
    }

    /// Do the writers of `krate`, a library's, take their `Formatter`'s
    /// options (ADR 0137)?
    pub(super) fn writers_take_options(&self, krate: rustc_span::def_id::CrateNum) -> bool {
        self.library(krate).is_some_and(|library| library.writers_take_options)
    }

    /// May any write of a library's fail, of a function whatever it
    /// returns, a writer's `write` too (#1072)? Then std's `write_all` of
    /// its writer may, and a dictionary's call, which may be any.
    pub(super) fn writes_may_fail(&self) -> bool {
        self.dependencies
            .libraries
            .values()
            .any(|library| library.writes_may_fail)
    }

    /// The program, compiled for its tests (`--tested`), whose dictionary
    /// of an impl of the trait of `konst`, a trait's constant, lacks it, as
    /// its own generic code reads it nowhere (ADR 0106): a crate that reads
    /// it of a type parameter may be given that dictionary, and is refused
    /// (#597). The program's name, and the impl's type.
    pub(super) fn lacks_dictionary_const(&self, konst: DefId) -> Option<(&'a str, rustc_middle::ty::Ty<'tcx>)> {
        let tr = self.tcx.trait_of_assoc(konst)?;
        let konst_key = key(self.tcx, konst);
        self.tcx.all_impls(tr).find_map(|imp| {
            let library = self.library(imp.krate)?;
            let lacks = library.impls.contains(&key(self.tcx, imp))
                && (library.dictionary_consts.as_ref()).is_some_and(|consts| !consts.contains(&konst_key));
            lacks.then(|| {
                (
                    library.name.as_str(),
                    self.tcx.type_of(imp).instantiate_identity().skip_normalization(),
                )
            })
        })
    }

    /// The program, compiled for its tests, whose `dyn`s of its own trait
    /// `tr` drop nothing (#597): one a crate makes of what drops it would
    /// drop nothing of.
    pub(super) fn drops_no_dyn_of(&self, tr: DefId) -> Option<&'a str> {
        let library = self.library(tr.krate)?;
        library.dyns_drop_nothing.contains(&key(self.tcx, tr)).then_some(())?;
        Some(library.name.as_str())
    }

    /// Is `id` a library's trait impl, one its consumers call?
    pub(super) fn has_impl(&self, id: DefId) -> bool {
        !id.is_local()
            && !super::recognition::not_compiled_for_build(self.tcx, id)
            && self
                .library(id.krate)
                .is_some_and(|library| library.impls.contains(&key(self.tcx, id)))
    }

    /// What `id`, a library's type, is declared as: its module's `.d.ts`,
    /// by its JS file's specifier, and its name there, where its library's
    /// declarations export it (ADR 0210, amended).
    pub(super) fn declared_type(&self, id: DefId) -> Option<&'a (String, String, bool)> {
        if id.is_local() {
            return None;
        }
        let library = self.library(id.krate)?;
        library.types.get(&key(self.tcx, id))
    }

    /// Is `id`, a library's enum, told apart by its keys, as its contract
    /// says (ADR 1071)?
    pub(super) fn keyed(&self, id: DefId) -> bool {
        !id.is_local()
            && self
                .library(id.krate)
                .is_some_and(|library| library.keyed.contains(&key(self.tcx, id)))
    }

    /// Is `id`, a library's type, equal only to itself, as its contract
    /// says (ADR 0864)?
    pub(super) fn equal_only_to_itself(&self, id: DefId) -> bool {
        !id.is_local()
            && self
                .library(id.krate)
                .is_some_and(|library| library.identities.contains(&key(self.tcx, id)))
    }

    /// Is the closure `id`, a library's, cloned apart, as its contract says
    /// its library made it (ADR 0522, amended; #1202)? `None` of one it
    /// neither clones nor gives to clone, or of the crate's own, which no
    /// library's contract says.
    pub(super) fn closure_cloned_apart(&self, id: DefId) -> Option<bool> {
        (self.library(id.krate)?.cloned_closures.get(&key(self.tcx, id))).copied()
    }

    /// Is `==` of `id`, a library's type, its address's, as its contract
    /// says (ADR 0864, amended)?
    pub(super) fn eq_by_address(&self, id: DefId) -> bool {
        !id.is_local()
            && self
                .library(id.krate)
                .is_some_and(|library| library.eq_by_address.contains(&key(self.tcx, id)))
    }

    /// Is each call of `id`, a library's function, what it's given, as its
    /// contract says (ADR 0584, amended)?
    pub(super) fn gives_its_argument(&self, id: DefId) -> bool {
        !id.is_local()
            && self
                .library(id.krate)
                .is_some_and(|library| library.gives_argument.contains(&key(self.tcx, id)))
    }

    /// Is `imp`, a library's impl of `PartialOrd`, one whose `partial_cmp`
    /// is `Some(self.cmp(other))`, as its contract says, its own compile
    /// having read its HIR (#964)?
    pub(super) fn partial_cmp_is_cmp(&self, imp: DefId) -> bool {
        !imp.is_local()
            && self
                .library(imp.krate)
                .is_some_and(|library| library.partial_cmp_is_cmp.contains(&key(self.tcx, imp)))
    }

    /// Does `id`, a library's untagged enum, `Display` each variant's value
    /// as it is, as its contract says (#557, rd730)?
    pub(super) fn displays_its_value(&self, id: DefId) -> bool {
        !id.is_local()
            && self
                .library(id.krate)
                .is_some_and(|library| library.displays_value.contains(&key(self.tcx, id)))
    }

    /// Does a library's code make values of `id`, a struct, as its contract
    /// says: the program's own data (ADR 1022, amended)?
    pub(super) fn built(&self, id: DefId) -> bool {
        let key = key(self.tcx, id);
        (self.dependencies.libraries.values()).any(|library| library.built.contains(&key))
    }

    /// Is `krate` a library's, one rust-js built for this crate to use, not
    /// `@rust-js/std` (ADR 0100)?
    pub(super) fn is_library(&self, krate: rustc_span::def_id::CrateNum) -> bool {
        self.library(krate).is_some_and(|library| !library.builtin)
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

    /// The modules `@rust-js/std`'s items are imported from, the library
    /// built into the compiler (ADR 0561), whose exports a module reserves
    /// no name for where nothing imports them (#1156).
    pub(super) fn builtin_modules(&self) -> IndexSet<&'a str> {
        (self.dependencies.libraries.values())
            .filter(|library| library.builtin)
            .flat_map(|library| library.items.values())
            .map(|item| item.from.as_str())
            .collect()
    }

    /// Every import of an item a library's module re-exports (rd704), its
    /// file and what that exports it as, which `import_key` gives a module
    /// naming it through that module.
    pub(super) fn all_reexported(&self) -> impl Iterator<Item = &'a (String, String)> {
        (self.dependencies.libraries.values())
            .flat_map(|library| library.reexported.values().map(|(_, imported)| imported))
    }
}

/// The library items each module of the crate names through a module of
/// their library's, its `use`s' and its expressions' paths.
struct LibraryPaths {
    /// Each named through its library's root, by the module:
    /// `use react_isomorphic::CHILDREN;` of a `pub use
    /// react_client::CHILDREN;` there, a path whose last but one segment
    /// is the library's root (the user's decision, ADR 1008, amended).
    rooted: IndexSet<(LocalModId, DefId)>,
    /// Each named through another of the library's modules than its own,
    /// the first written: that module and the name the path gives it,
    /// `react_fiber_reconciler` and `default_on_caught_error` of `use
    /// react_reconciler::react_fiber_reconciler::default_on_caught_error;`
    /// (rd704).
    through: IndexMap<(LocalModId, DefId), (DefId, rustc_span::Symbol)>,
}

fn library_paths(tcx: TyCtxt<'_>) -> LibraryPaths {
    use rustc_hir::def::{DefKind, Res};
    use rustc_hir::intravisit::{self, Visitor};
    struct Paths<'tcx> {
        tcx: TyCtxt<'tcx>,
        found: LibraryPaths,
    }
    impl<'tcx> Visitor<'tcx> for Paths<'tcx> {
        type NestedFilter = rustc_middle::hir::nested_filter::All;
        fn maybe_tcx(&mut self) -> TyCtxt<'tcx> {
            self.tcx
        }
        fn visit_path(&mut self, path: &rustc_hir::Path<'tcx>, id: rustc_hir::HirId) {
            if let Res::Def(_, item) = path.res
                && !item.is_local()
                && let [.., parent, last] = path.segments
                && let Res::Def(DefKind::Mod, module) = parent.res
                && module.krate == item.krate
            {
                let at = (self.tcx.parent_module(id), item);
                if module == item.krate.as_def_id() {
                    self.found.rooted.insert(at);
                } else {
                    // The first path written in the module, its home's too,
                    // which re-exports nothing (rd704).
                    self.found.through.entry(at).or_insert((module, last.ident.name));
                }
            }
            intravisit::walk_path(self, path);
        }
    }
    let mut paths = Paths {
        tcx,
        found: LibraryPaths {
            rooted: IndexSet::default(),
            through: IndexMap::default(),
        },
    };
    tcx.hir_visit_all_item_likes_in_crate(&mut paths);
    paths.found
}
