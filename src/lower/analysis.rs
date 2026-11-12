//! Collect crate facts before emission; no function lowering or linking.

pub(super) mod arguments;
mod back_stepped;

mod ground;
pub(super) use ground::{Ground, of as ground};
mod hint_asked;
use crate::collections::{IndexMap, IndexSet};
pub(in crate::lower) use back_stepped::{BackStepped, SteppedLocal, field_of, made_by};
mod callers_decide;
mod cell_borrows;
mod cell_flow;
mod chars_asked;
mod vec_uses;
pub(super) use cell_borrows::{
    Borrow, Cell, CellBorrows, binding_call_runs_nothing, binding_runs_nothing, borrowed, may_be_cell,
    resolve_runs_nothing, unsized_in_place,
};
pub(super) use cell_flow::CellClass;
mod clones;
pub(in crate::lower) use clones::Cloned;
pub(in crate::lower) use hint_asked::{Hinted, asked_of_own};
mod closures;
mod collected;
use collected::{coded_inline_consts, lowered_derive};
pub use collected::{collect_bodies, collect_initializers, collect_mir, collect_ranges, collect_std_bodies};
pub(in crate::lower) use collected::{exported_value_const, impl_const, is_type_system_const};
mod debug;
mod drops;
mod fmt_failures;
mod fresh_strings;
pub(in crate::lower) use fmt_failures::{Failing, call_may_fail};
pub(in crate::lower) use fresh_strings::fresh_string_given;
pub(super) mod fn_pointers;
mod generic_consts;
pub(super) use fn_pointers::{Called, FnPointers, Holder};
mod held_adapters;
mod iterator_steps;
mod js_given;
pub(super) use js_given::{JsGiven, Params, flattened_alike, flattened_by_name, js_facing, read_by_flatten};
mod lazy_locals;
mod may_run;
mod mutation;
mod namespaces;
pub(super) use namespaces::{Through, module_paths, namespace_in, namespace_of};
mod naming;
pub(super) mod nulled_params;
pub(super) mod only_called;
pub(super) mod owned_by_return;
mod plain_locals;
mod programs;
mod rc_counts;
mod resolvers;
mod scopes;
pub(super) use scopes::Scopes;
mod self_handles;
mod shown_params;
pub(super) use self_handles::SelfHandles;
mod stored_places;
pub(in crate::lower) use stored_places::StoredPlaces;
mod tests;
mod throwing;
mod unwinding;
pub(in crate::lower) use unwinding::{never_unwinds_given, std_quiet};
mod wholes;
pub(super) use wholes::{Wholes, whole_locals};

pub(super) use foreign_bodies::foreign_bodies;
pub(super) use naming::{UsesWritten, receiver_name, renamed_in, self_box_name};
pub(super) use throwing::call_throws;
mod caller_locations;
mod foreign_bodies;
pub(in crate::lower) mod type_facts;
mod validation;

use super::bindings::{self, Export, is_binding};
use super::traits;
use super::{Body, FnInfo, module_path};
use crate::lower::recognition::{
    Recognition, StdItem, TypeFact, is_js_object_deref, is_std_def, is_std_reader_impl, is_unlowered_impl, known_derive,
};
pub(super) use closures::{ClosureFacts, borrowed_fixed_bindings, clone_shares, taken_by_value, writes_what_it_took};
pub(super) use debug::{FormatCall, format_calls};
pub(super) use debug::{WriterFacts, Writes};
use drops::drop_params;
use generic_consts::generic_consts;
pub(super) use mutation::clones_by_itself;
pub(super) use mutation::{clone_bounded, clones_itself, copies_itself, copy_bounded};
use mutation::{cloned_params, copied_params, mutated_types, unboxed_params};
use naming::{
    exported_across_modules, js_uses, local_consts, local_functions, name_imports, name_items, named_expressions,
    refuse_two_names, required_uses,
};
pub(super) use plain_locals::{holds_a_value, module_statics, refuse_boxed_export};
use rc_counts::counted_rcs;
pub(super) use rc_counts::{Counted, Head, head};
use rustc_hir::def::DefKind;
use rustc_hir::find_attr;
use rustc_middle::thir::ExprKind;
use rustc_middle::ty::{self, Ty, TyCtxt, TypeVisitableExt};
use rustc_span::Symbol;
use rustc_span::def_id::{DefId, LocalDefId, LocalModId};
pub(super) use shown_params::{Shown, ShownParams};
use std::collections::BTreeMap;
use type_facts::type_fact_params;
use validation::{
    in_thread_local, reject_flatten_misuse, reject_namespace_value_reads, reject_parameter_name_misuse,
    reject_rest_misuse, reject_static_references, reject_unsupported,
};
pub(super) use validation::{init_of, is_thread_local};

/// The `const`s written in a function's body, each a `const` there (ADR
/// 0031, amended): by each, the block it's in and the bodies that read it.
pub(in crate::lower) type LocalConsts = IndexMap<DefId, (rustc_span::Span, IndexSet<DefId>)>;

/// Made by serde's `#[derive(Serialize)]` or `#[derive(Deserialize)]`, or
/// inside what they made (its `const _: () = { .. }`): left out, since
/// rust-js writes each type's JSON codec itself (ADR 0077).
pub(super) use super::recognition::from_serde_derive;

use super::recognition::serde_derived;
/// Is `id` serde's derived `impl Serialize` (`Some(true)`) or `impl
/// Deserialize` (`Some(false)`)?
/// Only the impls for the crate's own types: the derive's helpers inside
/// its `const _` block have impls too.
pub(super) use super::recognition::serde_impl;

/// Crate-wide facts collected before function emission. Body references point
/// into the captured THIR; all collections are owned by this analysis result.
pub(super) struct AnalyzedCrate<'a, 'tcx> {
    pub bodies: Vec<&'a Body<'tcx>>,
    /// What its flattened fields hold as their parents' own keys
    /// (`bindings::flattened`).
    pub flattened: bindings::Flattened,
    pub closures: IndexMap<DefId, &'a Body<'tcx>>,
    pub trait_impls: Vec<DefId>,
    pub dictionaries: Vec<DefId>,
    pub import_names: IndexMap<LocalModId, IndexMap<Export, String>>,
    pub imported: BTreeMap<Export, IndexSet<LocalModId>>,
    /// The exports read through `require(module)` (ADR 0305).
    pub required: IndexSet<Export>,
    /// Each library's function a module's `use` reads through
    /// `require(module)` (rd684, ADR 0305, amended).
    pub required_uses: IndexSet<(LocalModId, DefId)>,
    pub thread_local_inits: IndexMap<LocalDefId, LocalDefId>,
    pub consts: Vec<LocalDefId>,
    /// Each variant of a numbered enum, its module's `const` of its number,
    /// read by its name (ADR 0722, amended).
    pub variants: Vec<LocalDefId>,
    pub codecs: Vec<DefId>,
    pub modules: Vec<LocalModId>,
    pub taken: IndexMap<LocalModId, IndexSet<String>>,
    pub fns: IndexMap<DefId, FnInfo>,
    /// Keep collecting lowering diagnostics after item-name validation fails.
    pub failed: bool,
    pub called_from_elsewhere: IndexSet<DefId>,
    pub tests: crate::program::Tests,
    pub paths: IndexMap<LocalModId, Vec<String>>,
    pub mutated: IndexSet<Ty<'tcx>>,
    /// The `Rc`s counted, whose counts the crate reads (ADR 0320).
    pub counted: Counted<'tcx>,
    /// Each closure's handle on itself, or on what owns it, of those uncounted (ADRs 0843, 0317).
    pub self_handles: SelfHandles<'tcx>,
    /// Each function's scope struct, whose methods are functions declared
    /// in it (ADR 1043).
    pub scopes: Scopes<'tcx>,
    /// What each body may borrow, of its cells (ADR 0580).
    pub cell_borrows: CellBorrows<'tcx>,
    /// Which results and fields are whole numbers (rd537, ADR 0994).
    pub wholes: Wholes,
    /// Whether a function holding a value with a destructor is made a `dyn
    /// Fn`, which then drops it: a library's may be (ADR 0521).
    pub closure_facts: ClosureFacts<'tcx>,
    /// The types the crate clones, and those their clones clone (ADR 0525).
    pub cloned: clones::Cloned<'tcx>,
    /// The kinds of std's iterator adapters the crate's types hold (ADR
    /// 0542), which `held_cycle` asks of a `Cycle`, the one of them that's
    /// explicit wherever the crate makes one (ADR 0544): each other kind is
    /// std's struct where it's held (ADR 0871), whatever this says of it.
    pub explicit_adapters: IndexSet<DefId>,
    /// The map and filter types the crate steps from their back, each made
    /// knowing its back (ADR 0545).
    pub back_stepped: BackStepped<'tcx>,
    /// The `take(n)`s, `skip(n)`s and `step_by(n)`s whose hint generic
    /// code asks of what its callers give, each made `$taking`, `$skipping`
    /// or `$stepBy` where it's made (`hint_asked.rs`, #1236).
    pub hinted: Hinted<'tcx>,
    /// Whether the crate asks a string's `Chars` its rest, its `Debug`, or
    /// makes a `dyn` of one, so each is `$chars(s)`, not JS's own string
    /// iterator (#1186).
    pub chars_whole: bool,
    pub changed_vecs: IndexSet<Ty<'tcx>>,
    /// The types whose objects say where they're stored (#1113).
    pub stored_places: StoredPlaces<'tcx>,
    /// Each generic function's type parameters it's given a drop function
    /// for (ADR 0098), by their indices.
    pub drop_params: IndexMap<DefId, Vec<u32>>,
    /// The type parameters whose `Copy` bound takes a copy function (ADR 0289).
    pub copied: IndexSet<(DefId, u32)>,
    /// The type parameters whose `Clone` bound takes its dictionary (ADR
    /// 0914, amended).
    pub clones: IndexSet<(DefId, u32)>,
    /// The type parameters a `Some` of which is the value, no caller giving
    /// what may look like `None` (ADR 0051, amended).
    pub unboxed: IndexSet<(DefId, u32)>,
    /// What each function's JS takes its type parameters as, and the copies
    /// its calls need besides (ADR 0051, amended).
    pub js_given: JsGiven,
    /// The type parameters given as they are whose `Iterator` dictionary,
    /// or an extension's, is given, as they're stepped by it (ADRs 0411,
    /// 0487), with the trait's id.
    pub stepped: IndexSet<(DefId, u32, DefId)>,
    /// Each generic function's type parameters it's given a size, an
    /// alignment or a name of (ADR 0145), by their indices.
    pub type_facts: IndexMap<DefId, Vec<(u32, TypeFact)>>,
    /// The types built of each one's slots it's given a layout of, past its associated types' (#737).
    pub built_slots: crate::lower::traits::BuiltSlots<'tcx>,
    /// What its traits' dictionaries carry of std supertraits' items (#990).
    pub carried_super: type_facts::CarriedSuper,
    /// The functions that may return `Err(fmt::Error)` (ADR 0187).
    pub failing: IndexSet<DefId>,
    /// Whether any write of the crate's or its libraries' may fail, of a
    /// function whatever it returns, a writer's `write` too (#1072).
    pub writes_may_fail: bool,
    /// Whether a write through a dictionary may, a generic `T`'s: in a
    /// library, not where only what no consumer compiles may (#1392).
    pub dictionaries_fail: bool,
    /// The throwing functions, whose result is a throwing JS call's (ADR 0644).
    pub throwing: IndexSet<DefId>,
    /// The functions that never unwind, whose call throws only where what
    /// it's given does (ADR 0902); and those whose calls can't be seen (#925).
    pub never_unwinding: IndexSet<DefId>,
    pub quiet_fns: IndexSet<DefId>,
    /// The functions that take their caller's location (ADR 0427).
    pub caller_locations: IndexSet<DefId>,
    /// Whether the crate asks a `Vec` its capacity (ADR 0419).
    pub asks_capacity: bool,
    pub generic_consts: IndexSet<DefId>,
    /// What its writers take beyond their arguments, and which of its
    /// functions read a `Formatter`'s options (ADRs 0058, 0137, 0559).
    pub writer_facts: WriterFacts,
    /// The type parameters each function shows with options, and each type it gives one (#1331).
    pub shown_params: ShownParams<'tcx>,
    /// Whether it reads a function's identity, of which its libraries'
    /// JS may merge two where Rust has two (`Foreign::merges_identities`,
    /// #1052).
    pub reads_fn_identity: bool,
    /// The thread-locals only read and set in their own module, which
    /// `plain_statics` makes plain: what their borrows' closures are given is
    /// what's in them, as nothing borrows them meanwhile (ADR 0328).
    pub quiet_locals: IndexSet<LocalDefId>,
    /// A library's thread-locals none of whose borrows here can meet
    /// another, so none is counted (ADR 0863, amended, gap rd76).
    pub quiet_library_keys: IndexSet<DefId>,
    /// The thread-locals, by their keys, made where they're first used, as
    /// their module's load can't make them first (ADR 0037, amended): each
    /// `$local(() => init)`, read by calling it.
    pub lazy_locals: IndexSet<LocalDefId>,
    /// The others, each made as its module loads, and what its initializer
    /// may run and read of the crate's there; `None`, what rust-js can't
    /// follow (ADR 1006, amended).
    pub locals_at_load: IndexMap<LocalDefId, Option<Vec<DefId>>>,
    /// The functions a block makes and gives, each a named function
    /// expression there (ADR 0296).
    pub named_expressions: IndexSet<DefId>,
    /// The functions written in a function's body, each a function
    /// declaration there (ADR 0308).
    pub local_functions: IndexMap<DefId, rustc_span::Span>,
    /// The `const`s written in a function's body, each a `const` there,
    /// by the block they're in and the bodies that read them (ADR 0031,
    /// amended).
    pub local_consts: LocalConsts,
    /// What each module holds of a binding, and where a path reads one
    /// through it (ADR 0633).
    pub held: bindings::Held,
    /// Where a path names a function through the module that has it,
    /// read through its namespace (rd114, ADR 1018).
    pub module_paths: IndexMap<rustc_span::Span, Through>,
    /// The fields a `js::null()` is written to, whose `.d.ts` says `| null`
    /// (ADR 0935, amended).
    pub null_fields: IndexSet<DefId>,
}

pub(super) fn analyze_crate<'a, 'tcx>(
    tcx: TyCtxt<'tcx>,
    ground: Ground<'a, 'tcx>,
    all_bodies: &'a [Body<'tcx>],
    initializers: &[Body<'tcx>],
    library: bool,
    (instances, made_where_used): (&super::Instances, &IndexSet<LocalDefId>),
) -> Option<AnalyzedCrate<'a, 'tcx>> {
    // What it stands on: its bodies but rustc's test harness's, its
    // libraries and its trait impls (`ground.rs`), made before the bodies
    // were told which adapters each makes in place.
    let all_bodies = ground.harness.left_out(tcx, all_bodies);
    let Ground {
        harness: ground::Harness { markers, .. },
        writer_facts,
        shown_params,
        reads_fn_identity,
        foreign,
        trait_impls,
        cloned,
        asks_capacity,
    } = ground;
    if !bindings::validate(tcx)
        || !super::declarations::validate_types(tcx)
        || !traits::validate(tcx, &foreign)
        || !super::jsx_api::validate(tcx)
        || !super::untagged::validate(tcx, &foreign)
        || !foreign.check()
    {
        return None;
    }

    let flattened = bindings::flattened(tcx, &all_bodies, &foreign);
    if !reject_unsupported(tcx, &foreign, &markers)
        || !reject_static_references(tcx, &all_bodies)
        || !reject_namespace_value_reads(tcx, &all_bodies)
        || !reject_rest_misuse(tcx)
        || !reject_parameter_name_misuse(tcx)
    {
        return None;
    }
    // A class's misuse, which asks whether it's equal only to itself, of
    // its impls (ADR 0761, amended).
    let classes_recognition = Recognition {
        tcx,
        typing_env: ty::TypingEnv::fully_monomorphized(),
        trait_impls: &trait_impls,
        foreign: &foreign,
    };
    // A flattened field's misuse, which asks what its cell's own calls are
    // (ADR 1044, amended).
    if !reject_flatten_misuse(&classes_recognition, &all_bodies, &flattened)
        || !super::classes::reject_misuse(&classes_recognition, &all_bodies)
    {
        return None;
    }
    if !super::classes::reject_constructors_as_values(tcx, &all_bodies) {
        return None;
    }
    if !super::bindings::reject_made_in_place(tcx, &all_bodies)
        || !super::bindings::reject_attributed_calls(tcx, &all_bodies)
        || !super::bindings::reject_nullable_held(&classes_recognition, &all_bodies)
        || !super::bindings::reject_unkept_collections(&classes_recognition)
        || !super::bindings::reject_unread_parameters(tcx)
    {
        return None;
    }
    // The impls that get a dictionary: not `From`'s (ADR 0052), nor serde's,
    // whose evidence is a codec (ADR 0081), nor a JS object's `Deref` to the
    // class it extends, which is never lowered: its dictionary is std's,
    // the object itself (ADRs 0024, 0394).
    let dictionaries: Vec<DefId> = trait_impls
        .iter()
        .copied()
        .filter(|&id| {
            traits::operational(
                tcx,
                &foreign,
                tcx.impl_trait_ref(id)
                    .instantiate_identity()
                    .skip_normalization()
                    .def_id,
            )
        })
        .filter(|&id| serde_impl(tcx, id).is_none())
        .filter(|&id| !is_js_object_deref(tcx, id))
        // Nor a crate's own `BufRead` or `Seek`, never lowered (ADR 0425,
        // #1489); std's `Chain`'s and `Take`'s are (#1156), and a crate's
        // own `Read`, no longer `unlowered_trait`'s (#1297).
        .filter(|&id| !is_unlowered_impl(tcx, id) || is_std_reader_impl(tcx, id))
        // A class's `Display`, its prototype's `toString` (#358).
        .filter(|&id| super::classes::displayed_class(tcx, id).is_none())
        .collect();

    // Closures are lowered inside the function that creates them.
    let (bodies, closures): (Vec<&Body<'tcx>>, Vec<&Body<'tcx>>) = all_bodies
        .iter()
        .partition(|body| matches!(tcx.def_kind(body.def_id), DefKind::Fn | DefKind::AssocFn));
    let all_bodies = &all_bodies;
    let closures: IndexMap<DefId, &Body<'tcx>> = closures.into_iter().map(|b| (b.def_id, b)).collect();

    // What each module holds of a binding, and where a path reads one
    // through it (ADR 0633).
    let held = bindings::held(tcx, &foreign);
    // A static's or a constant's initializer, lowered as code, `static F:
    // fn() -> f64 = toast`, imports what it reads as a function does.
    let with_initializers: Vec<&Body<'tcx>> = all_bodies.iter().copied().chain(initializers).collect();
    let mut uses = js_uses(tcx, &with_initializers, &held, instances);
    // What the crate's libraries export (ADR 0100) is named before lowering,
    // after its name there, a module's default too, `import noop from
    // "../shared/noop.js"` (ADR 0773), and imported by the modules that turn
    // out to use it.
    // By its package's name too, where its root re-exports it, as a
    // module naming it through its root imports it (ADR 1008, amended).
    for imported in foreign.all() {
        let by_file = (imported.from.clone(), imported.export.clone());
        for export in std::iter::once(by_file).chain(imported.rooted.clone()) {
            uses.bound_to.entry(export.clone()).or_default();
            uses.imported.entry(export.clone()).or_default();
            uses.named.insert(export, imported.name.clone());
        }
    }
    // And through a library's module that re-exports it, by what that
    // module's file exports it as (rd704).
    for export in foreign.all_reexported() {
        uses.bound_to.entry(export.clone()).or_default();
        uses.imported.entry(export.clone()).or_default();
        uses.named.insert(export.clone(), export.1.clone());
    }
    // A library's module a path names a function through, imported whole,
    // `import * as Scheduler from "scheduler"` (rd114, ADR 1018): from where
    // the module the path is in imports the function, by its name.
    let module_paths = module_paths(tcx, &foreign);
    for through in module_paths.values().filter(|through| !through.function.is_local()) {
        if let Some((from, export)) = foreign.import_key(through.function, through.scope)
            && export != "default"
        {
            let export = (from, crate::js::NAMESPACE.to_string());
            uses.bound_to.entry(export.clone()).or_default();
            uses.imported.entry(export.clone()).or_default();
            uses.named.insert(export, through.namespace.clone());
        }
    }
    // A library's `#[rust_js::namespace]` a body writes, its module
    // imported whole, by the struct's name, from where its functions are
    // (ADR 1037).
    let namespaces: IndexSet<DefId> = (with_initializers.iter())
        .flat_map(|body| body.thir.exprs.iter())
        .filter_map(|expr| match expr.ty.peel_refs().kind() {
            ty::Adt(adt, _) if !adt.did().is_local() && super::classes::is_namespace(tcx, adt.did()) => Some(adt.did()),
            _ => None,
        })
        .collect();
    for adt in namespaces {
        let methods =
            (tcx.inherent_impls(adt).iter()).flat_map(|&imp| tcx.associated_item_def_ids(imp).iter().copied());
        if let Some(from) = methods
            .filter_map(|item| foreign.item(item))
            .map(|item| item.from.clone())
            .next()
        {
            let export = (from, crate::js::NAMESPACE.to_string());
            uses.bound_to.entry(export.clone()).or_default();
            uses.imported.entry(export.clone()).or_default();
            uses.named.insert(export, tcx.item_name(adt).to_string());
        }
    }
    // The binding's class a class extends, imported where the class is,
    // `import { Component } from "react"` of `class Surface extends
    // Component` (rd331).
    for adt in tcx.hir_crate_items(()).definitions().map(|def| def.to_def_id()) {
        if tcx.def_kind(adt) == DefKind::Struct
            && let Some((imp, export)) = super::classes::superclass(tcx, adt)
            && let Some(home) = super::classes::home(tcx, adt)
        {
            uses.bound_to
                .entry(export.clone())
                .or_default()
                .insert(tcx.impl_trait_id(imp));
            uses.imported.entry(export).or_default().insert(home);
        }
    }
    // A binding a library's module holds, named after its `const` there.
    for held in foreign.all_held() {
        let export = (held.from.clone(), held.export.clone());
        uses.bound_to.entry(export.clone()).or_default();
        uses.imported.entry(export.clone()).or_default();
        uses.named.insert(export, held.name.clone());
    }

    // `const` items (ADR 0031) and statics (ADR 0096), with the values rustc
    // has computed. One in a function goes beside it, in its module.
    let consts: Vec<LocalDefId> = tcx
        .hir_crate_items(())
        .definitions()
        .filter(|&d| match tcx.def_kind(d) {
            // Not a `js::import!`'s, which rust-js reads (ADR 0110).
            DefKind::Const { .. } => {
                !markers.iter().any(|&(m, _)| m == d)
                    && !from_serde_derive(tcx, d)
                    && !bindings::is_mark(tcx, d.to_def_id())
            }
            // Not std's storage for a thread-local, which JS needs none of.
            DefKind::Static { .. } => !tcx.is_foreign_item(d) && in_thread_local(tcx, d).is_none(),
            // An impl's own, of a type, not a dictionary's (ADR 0176): a
            // constant of its module too (ADR 0410); a class's, its
            // prototype's property, whoever reads it (ADR 0761).
            DefKind::AssocConst { .. } => {
                impl_const(tcx, d.to_def_id())
                    || exported_value_const(tcx, d.to_def_id())
                    || super::classes::member(tcx, d.to_def_id()) == Some(super::classes::Member::Const)
            }
            _ => false,
        })
        .chain(coded_inline_consts(tcx, all_bodies))
        .collect();
    // Each variant of a numbered enum, `Status::Pending`, its module's
    // `const Pending = 0;`, read by its name (ADR 0722, amended).
    let variants: Vec<LocalDefId> = tcx
        .hir_crate_items(())
        .definitions()
        .filter(|&d| tcx.def_kind(d) == DefKind::Enum && bindings::numbered(tcx, tcx.adt_def(d)))
        .flat_map(|d| tcx.adt_def(d).variants().iter().filter_map(|v| v.def_id.as_local()))
        .collect();

    // A derived `Serialize`'s `serialize` and `Deserialize`'s `deserialize`,
    // which rust-js writes (ADRs 0077 and 0078).
    // A derived serde impl's method is a codec rust-js writes; a hand-written
    // one's is its body (ADR 0459).
    let codecs: Vec<DefId> = trait_impls
        .iter()
        .filter(|&&id| serde_impl(tcx, id).is_some() && serde_derived(tcx, id))
        .map(|&id| tcx.associated_item_def_ids(id)[0])
        .collect();
    // What gets a JS name: functions and methods, `const`s, a numbered
    // enum's variants, dictionaries, and codecs.
    let items: Vec<DefId> = bodies
        .iter()
        .map(|body| body.def_id)
        .chain(consts.iter().map(|id| id.to_def_id()))
        .chain(variants.iter().map(|id| id.to_def_id()))
        .chain(dictionaries.iter().copied())
        .chain(codecs.iter().copied())
        .collect();
    // Each thread-local's `init` function: lowered like any function, its
    // body is the variable's value. Or, of a `const { .. }` one, the `const`
    // in its block, whose value is.
    let thread_local_inits: IndexMap<LocalDefId, LocalDefId> = bodies
        .iter()
        .filter_map(|body| body.def_id.as_local())
        .filter(|&d| tcx.def_kind(d) == DefKind::Fn)
        .chain(
            consts
                .iter()
                .copied()
                .filter(|&d| matches!(tcx.def_kind(d), DefKind::Const { .. })),
        )
        .filter_map(|d| Some((d, init_of(tcx, d)?)))
        .collect();

    // Each `Rc` whose counts are read, or whose pointee's drop runs a
    // destructor (ADR 0320); and each a library that counts it hands the
    // crate, which share their counts (ADRs 0523, 0626, 0852). A library's
    // type parameters are its consumers' types, any of which may have a
    // destructor.
    let params_may_drop = crate::lower::traits::may_have_destructors(tcx, (&foreign, library));
    let reached = foreign.reached(all_bodies);
    let recognition = Recognition {
        tcx,
        typing_env: ty::TypingEnv::fully_monomorphized(),
        trait_impls: &trait_impls,
        foreign: &foreign,
    };
    // The fields a `js::null()` is written to, declared `| null` (ADR 0935,
    // amended).
    let null_fields: IndexSet<DefId> = (all_bodies.iter().copied().chain(initializers))
        .flat_map(|body| super::body_queries::null_written_fields(tcx, &body.thir))
        .collect();
    // Each thing named in JS, the crate's functions among them.
    let item_set: IndexSet<DefId> = items.iter().copied().collect();
    // What dropping `ty` runs, of `counted`'s `Rc`s, asked where `owner` is,
    // as a type it gives may name its own type parameters (ADR 0451).
    let typing_env_where = |ty: Ty<'tcx>, owner: DefId| match ty.has_param() {
        true => ty::TypingEnv::non_body_analysis(tcx, owner),
        false => ty::TypingEnv::fully_monomorphized(),
    };
    let runs_where = |closure_facts: &ClosureFacts<'tcx>, counted: &Counted<'tcx>, ty: Ty<'tcx>, owner: DefId| {
        let recognition = Recognition {
            typing_env: typing_env_where(ty, owner),
            ..recognition
        };
        super::drops::drop_runs(recognition, (library, closure_facts), counted, ty)
    };
    // The `Rc`s counted, and the type parameters given a drop, together: a
    // program's `Rc` of a value of a type parameter is counted where a
    // caller gives it one whose drop runs, which counting more may make
    // more of; both only grow, so they settle (ADR 0914, amended). A
    // library's, of any its consumers may give, whose `Rc`s share its
    // counts (ADR 0852); but not one of another library's type that it
    // counts none of, which that library's contract decides for every
    // consumer, refusing one that would count it: only what the crate's
    // own calls give (ADR 0852, amended).
    // Each closure's handle on itself or on what owns them, whose upgrades
    // ask no count (ADRs 0843, 0317, amended), a library's owner by contract.
    let mut self_handles = SelfHandles::new(tcx, all_bodies, &|id, index| foreign.owned_by_return(id, index));
    // The types the crate clones, `ground`'s: which closures an `Rc<dyn Fn>`
    // may be made of twice too (#1249).
    let counted_with = |closure_facts: &ClosureFacts<'tcx>| {
        let mut given: IndexSet<(DefId, u32)> = IndexSet::default();
        loop {
            let counted = counted_rcs(
                tcx,
                (all_bodies, &self_handles, &cloned, library),
                (params_may_drop, &reached.counted_rcs, &|owner, index, pointee| {
                    library && !foreign.counts_no_rc_of_its_type(pointee) || given.contains(&(owner, index))
                }),
                |counted, pointee| super::drops::drop_runs(recognition, (library, closure_facts), counted, pointee),
            );
            let runs = |ty, owner| runs_where(closure_facts, &counted, ty, owner);
            let more: IndexSet<(DefId, u32)> = (drop_params(tcx, all_bodies, &item_set, &foreign, library, runs))
                .into_iter()
                .flat_map(|(id, indices)| indices.into_iter().map(move |index| (id, index)))
                .collect();
            if more.is_subset(&given) {
                return (counted, given);
            }
            given.extend(more);
        }
    };
    // Whether a function holding a value with a destructor is made a `dyn
    // Fn`, which then drops it (ADR 0521): a library's may be. A captured
    // `Rc` counted may make one, and a `dyn Fn` that drops may count an
    // `Rc`; both only grow, so a second pass settles them.
    // The kinds of adapters the crate's types hold (ADR 0542), of which a
    // `Cycle`'s is asked (`held_cycle`).
    let explicit_adapters = held_adapters::held_adapters(tcx);
    let mut closure_facts = ClosureFacts::new(tcx, all_bodies, &cloned, library);
    let (mut counted, given) = counted_with(&closure_facts);
    // Whether the crate makes a `dyn Fn` of a function holding a value with
    // a destructor, or a library hands it one whose contract says its own
    // may (ADR 0626, amended): what its `dyn Fn`s drop by, a library's as a
    // program's, and what its contract says of them. A consumer's closure
    // given a library's generic function that holds what drops is its own
    // instance's, or refused (ADR 0914). A program's of a type parameter's
    // value drops where a caller gives it one whose drop runs (#580); a
    // library's callers are its consumers, which it can't see.
    let library_drops = (reached.fn_dyns_of.iter()).any(|name| foreign.drops_fn_dyns(name));
    let given_a_drop = |owner, index| given.contains(&(owner, index));
    let made = closures::dropping_fn_dyns(
        tcx,
        all_bodies,
        (params_may_drop, (!library).then_some(&given_a_drop), reached.functions),
        |ty| super::drops::drop_runs(recognition, (library, &closure_facts), &counted, ty),
    );
    closure_facts.fn_dyns_drop = closures::FnDynDrops::of(tcx, library_drops, &made);
    let drops = closure_facts.fn_dyns_drop.any();
    if drops {
        counted = counted_with(&closure_facts).0;
    }
    // A library whose `dyn Fn`s drop nothing that holds one the crate's
    // may be: it would drop nothing of one it's given (ADR 0626, amended).
    if drops && let Some(name) = (reached.fn_dyns_of.iter()).find(|name| !foreign.drops_fn_dyns(name)) {
        // Its crate's name, which the refusal says (#1130).
        let name = foreign.library_named(name);
        closures::refuse_dropping_fn_dyn(tcx, name, made.first().filter(|_| !library_drops), &self_handles);
        return None;
    }
    // A program its tests use whose `dyn`s of its own trait drop nothing
    // (ADR 1076, amended, #597).
    if programs::refuses_dropping_dyn(tcx, all_bodies, (&foreign, params_may_drop), |ty| {
        super::drops::drop_runs(recognition, (library, &closure_facts), &counted, ty)
    }) {
        return None;
    }
    // Whether a value with a destructor is made a `dyn Any`, which then
    // drops it (ADR 0691): what unwound, of a panic or of JS, drops nothing.
    // One that drops only by a `dyn Any` it holds doesn't, so one pass does.
    if closures::any_dyns_drop(
        tcx,
        all_bodies,
        (params_may_drop, foreign.makes_dropping_anys()),
        |ty| super::drops::drop_runs(recognition, (library, &closure_facts), &counted, ty),
    ) {
        closure_facts.any_dyns_drop = true;
        counted = counted_with(&closure_facts).0;
    }
    // And a `dyn Error`, and the `io::Error` that holds one (ADR 0462,
    // amended), as one pass does for a `dyn Any`.
    if closures::error_dyns_drop(
        &recognition,
        all_bodies,
        (params_may_drop, foreign.makes_dropping_errors()),
        |ty| super::drops::drop_runs(recognition, (library, &closure_facts), &counted, ty),
    ) {
        closure_facts.error_dyns_drop = true;
        counted = counted_with(&closure_facts).0;
    }
    // And a `Box<dyn Iterator>`, which carries the drop of what it owns
    // (ADR 0521, #1420), as one pass does for a `dyn Any`.
    if closures::iterator_dyns_drop(
        tcx,
        all_bodies,
        (params_may_drop, foreign.makes_dropping_iterators()),
        |ty| super::drops::drop_runs(recognition, (library, &closure_facts), &counted, ty),
    ) {
        closure_facts.iterator_dyns_drop = true;
        counted = counted_with(&closure_facts).0;
    }
    // A library that counts no `Rc` of a type hands the crate one, which the
    // two share as it is, the value itself: the crate can't count it either
    // (ADRs 0844, 0852).
    if let Some((pointee, name)) = (reached.uncounted_rcs.iter()).find(|&&(pointee, _)| counted.counts(tcx, pointee)) {
        tcx.dcx().err(format!(
            "rust-js: this crate counts an `Rc<{pointee}>`, whose counts it reads, whose last owner runs a destructor, or that may be one that does, but `{name}`, a rust-js library that counts none of them, hands it one, which both share as the value itself (ADR 0852)"
        ));
        return None;
    }
    // An untagged enum's variant of an `Rc` it counts, `{ value, strong,
    // weak }`, which JS can't tell by what it points at (ADR 0214).
    let mut told = true;
    for (span, held) in super::untagged::held_payloads(tcx) {
        if let Some(pointee) = super::recognition::rc_pointee(tcx, held)
            && counted.counts(tcx, pointee)
        {
            let why = "counted, `{ value, strong, weak }`, as its counts are read or its last owner runs a destructor";
            tcx.dcx().span_err(span, super::untagged::boxed_payload(held, why));
            told = false;
        }
    }
    if !told {
        return None;
    }
    // A handle on itself is no JS where nothing counts its `Rc`.
    self_handles.uncounted(|pointee| counted.counts(tcx, pointee));
    self_handles.unboxed(|pointee| super::representation::may_look_like_none(recognition.tcx, pointee));
    // A function's scope struct, of an `Rc` counted none (ADR 1043).
    let mut scopes = Scopes::new(tcx, all_bodies);
    scopes.retain(|s| !counted.counts(tcx, tcx.type_of(s.adt).instantiate_identity().skip_normalization()));
    // What it steps from its back (ADR 0545, #814, #851), and of an array's
    // items with a destructor, which its native forms don't step.
    // A `Vec`'s capacity kept as std grows it where the crate, or a library it uses, asks it (ADR 0419).
    let keeps_capacity = asks_capacity || foreign.keeps_capacity();
    let drop_runs = |item| super::drops::drop_runs(recognition, (library, &closure_facts), &counted, item);
    let mut back_stepped = back_stepped::of_crate(
        tcx,
        recognition,
        all_bodies,
        (library, drop_runs, keeps_capacity),
        (params_may_drop, (!library).then_some(&given_a_drop)),
        &cloned,
    );
    // What generic code asks the hint of, made knowing it where it's made,
    // by the makers of what's stepped from its back (#1236).
    let hint_asked = hint_asked::of_crate(tcx, recognition, all_bodies, keeps_capacity);
    back_stepped.typed.extend(hint_asked.knowing);
    let chars_whole = chars_asked::asks_chars_whole(tcx, recognition, all_bodies, (library, keeps_capacity));
    // What generic code compares by `eq` of two `TrustedLen` iterators
    // given it, which it can't count: refused where comparing runs code
    // (#386).
    callers_decide::refuse_what_callers_decide(tcx, recognition, all_bodies, |item| {
        super::drops::drop_runs(recognition, (library, &closure_facts), &counted, item)
    });
    // A closure cloned apart, made by a function of what it holds, where a
    // clone of it isn't the closure itself (ADR 0522). Of a library's, as
    // its library decided, which made it so, where it did (#1202): its
    // captures are its own compile's to see, and rustc infers one `FnMut`
    // by the bound it's given to, `map(|s| s.len())`.
    let cloned_apart = closure_facts.find_cloned_apart(|id| {
        if let Some(apart) = foreign.closure_cloned_apart(id) {
            return !apart;
        }
        let ty = tcx.type_of(id).instantiate_identity().skip_normalization();
        let typing_env = ty::TypingEnv::post_analysis(tcx, id);
        closures::clone_shares(tcx, typing_env, ty, |t| {
            super::drops::drop_runs(recognition, (library, &closure_facts), &counted, t)
        })
    });
    closure_facts.cloned_apart = cloned_apart;
    // What drops, of the `Rc`s counted and the closures' facts, which no
    // longer change: each type's answer found once.
    let dropping = super::drops::DropRuns::new(recognition, (library, &closure_facts), &counted);
    let runs_settled = |ty: Ty<'tcx>, owner: DefId| dropping.runs(typing_env_where(ty, owner), ty);
    // The type parameters whose `Clone` bound takes its dictionary: those a
    // caller gives what isn't its own clone (ADR 0914, amended).
    let clones = cloned_params(tcx, all_bodies, &item_set, &foreign, &counted);
    // The type parameters a `Some` of which is the value: those no caller
    // gives what may look like `None` (ADR 0051, amended).
    let unboxed = unboxed_params(&recognition, all_bodies, &item_set);

    // What each body may borrow, of the crate's cells (ADR 0580).
    let cell_borrows = CellBorrows::of_crate(
        tcx,
        &Recognition {
            tcx,
            typing_env: ty::TypingEnv::fully_monomorphized(),
            trait_impls: &trait_impls,
            foreign: &foreign,
        },
        (all_bodies, &clones),
        &|ty| dropping.runs(recognition.typing_env, ty),
    );
    let wholes = Wholes::of_crate(tcx, all_bodies, &thread_local_inits, &|adt| {
        cell_borrows.pointers().js_may_make(adt)
    });
    // What `with_borrow` runs while its thread-local is borrowed, that can't
    // ask whether it is (ADRs 0328, 0587): borrows nothing that may be it,
    // held as it's held, a `with_borrow`'s shared, which only a mutable
    // borrow asks of, and a `with_borrow_mut`'s or a `with`'s mutably (ADR
    // 0580, amended).
    let quiet = |given: plain_locals::Given<'tcx>, key: DefId, mutable: bool| {
        let held = Borrow::of(Cell::Key(key), mutable);
        match given {
            plain_locals::Given::Closure(closure) => {
                closures.contains_key(&closure.to_def_id()) && !cell_borrows.body_asks(tcx, closure.to_def_id(), held)
            }
            // A function given, as a call of it asks.
            plain_locals::Given::Fn(f, args) => !cell_borrows.call_asks(
                &Recognition {
                    tcx,
                    typing_env: ty::TypingEnv::fully_monomorphized(),
                    trait_impls: &trait_impls,
                    foreign: &foreign,
                },
                (key, f, args),
                None,
                held,
                &|ty| dropping.runs(recognition.typing_env, ty),
            ),
        }
    };
    // A thread-local is a cell, its module's own made plain after
    // (`plain_statics`), its quiet borrows given what's in it (ADR 0364).
    // Those made where they're first used, which aren't plain: a variable
    // can't be made as it's read (ADR 0037, amended).
    // What each body may run of the crate's, the one rule (`may_run`).
    let std_runs = |id, args| {
        let recognition = Recognition {
            tcx,
            typing_env: ty::TypingEnv::fully_monomorphized(),
            trait_impls: &trait_impls,
            foreign: &foreign,
        };
        recognition
            .is_std(id)
            .then(|| cell_borrows.std_runs(&recognition, id, args))
    };
    let runs = may_run::Runs::new(
        tcx,
        all_bodies,
        (&thread_local_inits, closures::handed_fn_dyns(tcx, all_bodies)),
        (cell_borrows.pointers(), &std_runs, &runs_settled),
    );
    let lazy_locals::LazyLocals {
        lazy: lazy_locals,
        at_load: locals_at_load,
    } = lazy_locals::lazy_locals(
        tcx,
        (&thread_local_inits, made_where_used),
        (&runs, all_bodies),
        &counted,
    );
    // What a function whose return alone owns a callback runs before it
    // returns that may call it (ADR 0317, amended, rd621).
    self_handles.quiet(tcx, &runs, &|id, index| foreign.quiet_by_return(id, index));
    let quiet_locals = plain_locals::plain_thread_locals(
        tcx,
        all_bodies,
        (thread_local_inits.values().copied()).filter(|key| !lazy_locals.contains(key)),
        &quiet,
    );
    let quiet_library_keys = plain_locals::quiet_library_keys(tcx, all_bodies, &quiet);
    // Another module's write of a thread-local of a value or a `static
    // mut`, its module's variable, which JS lets only that module set (ADRs
    // 0270, 0096, amended).
    let variable = |key: DefId| {
        foreign
            .item(key)
            .is_some_and(|item| item.calling.has(crate::library::ItemFact::Variable))
    };
    let refused = plain_locals::outside_writes(tcx, all_bodies, &variable, &quiet);
    for &(span, key) in &refused {
        let name = tcx.item_name(key);
        let message = match tcx.is_mutable_static(key) {
            true => format!(
                "rust-js: this writes `{name}`, or takes a pointer to it, from outside the module it's declared in, but `{name}` is its module's variable, which JS lets only that module set: write a `pub fn` in that module that sets it, and call that; another module reads it by its name (ADRs 0096, 0270, amended)"
            ),
            false => format!(
                "rust-js: this sets `{name}`, or hands out its cell, from outside the module it's declared in, but `{name}` is its module's variable, which JS lets only that module set: write a `pub fn` in that module that sets it, and call that; another module reads it with `get` or `with_borrow` (ADR 0270, amended)"
            ),
        };
        tcx.dcx().span_err(span, message);
    }
    if !refused.is_empty() {
        return None;
    }

    // The modules that get a JS file: every module with one of those, in the
    // order the first one appears. The root is no other: a root of `mod`s
    // alone, an app's list of its files, has none (ADR 0382).
    let mut modules = Vec::new();
    let mut seen_modules = IndexSet::default();
    for &def_id in &items {
        let module = super::body_module(tcx, instances, def_id);
        if seen_modules.insert(module) {
            modules.push(module);
        }
    }
    // And each that imports a file for what it does, `js::import!("./app.css")`
    // (ADR 0110), or re-exports another's function, `pub use`, a file of its
    // `export .. from` though it has nothing of its own (ADR 0240).
    let mut all_modules = Vec::new();
    tcx.hir_for_each_module(|module| all_modules.push(module));
    for module in all_modules {
        if bindings::marks(tcx, module, "import").next().is_some() && seen_modules.insert(module) {
            modules.push(module);
        }
        let reexports = tcx
            .hir_module_free_items(module)
            .any(|id| is_reexport(tcx, tcx.hir_item(id)))
            || super::reexports::globs_values(tcx, module);
        // Or a default that's another module's function (ADR 0240).
        let defaulted = || !super::default_exports::default_exports(tcx, module).is_empty();
        if (reexports || defaulted()) && seen_modules.insert(module) {
            modules.push(module);
        }
        // Or a type it exports, of a module of types alone a `.d.ts` (the
        // user's decision, ADR 1005).
        let types = tcx.hir_module_free_items(module).any(|id| {
            let def_id = id.owner_id.to_def_id();
            matches!(
                tcx.def_kind(def_id),
                DefKind::TyAlias | DefKind::Struct | DefKind::Enum | DefKind::Union
            ) && tcx.visibility(def_id).is_public()
                && !is_binding(tcx, def_id)
        });
        if types && seen_modules.insert(module) {
            modules.push(module);
        }
    }

    // The crate's own names first: an export is what its consumers and JS
    // call it by. An import is named around every one of them.
    // A function a block makes and gives is named in itself alone (ADR 0296).
    let named_expressions = named_expressions(tcx, all_bodies.iter().copied().chain(initializers));
    let local_functions = local_functions(tcx, all_bodies.iter().copied().chain(initializers), &named_expressions);
    // A `const` of a function's body, its function's (ADR 0031, amended).
    let local_consts = local_consts(
        tcx,
        all_bodies.iter().copied().chain(initializers),
        &consts,
        &local_functions,
    );
    // A library's function a `use` reads through `require(module)`, a
    // global no local takes (rd684, ADR 0305, amended).
    let (required_uses, failed_uses) = required_uses(tcx, &modules, &foreign);
    if !required_uses.is_empty() {
        uses.globals.insert("require".to_string());
    }
    let (mut taken, fns, failed) = name_items(
        tcx,
        &items,
        &modules,
        &uses.globals,
        &trait_impls,
        &named_expressions,
        instances,
    );
    let failed = failed | failed_uses | refuse_two_names(tcx, &modules, &foreign);
    let import_names = name_imports(tcx, &uses, &taken, &foreign);
    // Each import's name is the module's, which its own names keep apart
    // from; not `@rust-js/std`'s every export, which a module imports only
    // where its lowering reads it, by a name the linker keeps apart then:
    // std's `take` of `io::take` cost a crate's own `take` its name, `take$1`
    // (#1156).
    let builtin = foreign.builtin_modules();
    let of_a_library = |(from, _): &Export| !builtin.contains(from.as_str());
    for (module, names) in taken.iter_mut() {
        let imports = import_names[module].iter().filter(|(export, _)| of_a_library(export));
        names.extend(imports.map(|(_, name)| name.clone()));
    }
    let imported = uses.imported;
    let required = uses.required;
    let mut called_from_elsewhere = exported_across_modules(tcx, all_bodies, &fns, instances);
    let tests = tests::collect_tests(tcx, &markers, &bodies, &fns, &mut called_from_elsewhere);

    let paths: IndexMap<LocalModId, Vec<String>> = modules.iter().map(|&m| (m, module_path(tcx, m))).collect();

    let mutated = mutated_types(tcx, all_bodies);
    let changed_vecs = vec_uses::changed_vecs(tcx, all_bodies);
    let stored_places = stored_places::stored_places(tcx, all_bodies);
    // What a caller gives generic code whose drop runs something, a value
    // of a destructor, a counted `Rc`'s or a guard's: the one answer the
    // lowering's drops give (ADR 0451).
    // Asked where `owner` is, as a type it gives may name its own type
    // parameters, whose drops are passed on as they are.
    let runs = runs_settled;
    let drop_params = drop_params(tcx, all_bodies, &item_set, &foreign, library, runs);
    let copied = copied_params(tcx, all_bodies, &fns, &foreign);
    let stepped = iterator_steps::stepped_params(tcx, all_bodies, &fns, &foreign);
    // The functions JS calls, whose JS takes their type parameters as JS
    // gives them, and the copies their crate's calls need otherwise, each
    // named apart beside its function (ADR 0051, amended).
    let js_given = {
        let copyable = |id: DefId| {
            fns.get(&id).is_some_and(|info| {
                (info.owner.is_none() || crate::lower::classes::member(tcx, id).is_none())
                    && !local_functions.contains_key(&id)
                    && !named_expressions.contains(&id)
                    && !crate::lower::bindings::is_on_load(tcx, id)
            })
        };
        let mut name = |id: DefId, suffix: &str| {
            let info = &fns[&id];
            let base = format!("{}{suffix}", info.name);
            match &info.owner {
                // A method's, apart from its type's others.
                Some(owner) => {
                    let mut names: IndexSet<String> = (fns.values())
                        .filter(|other| other.module == info.module && other.owner.as_ref() == Some(owner))
                        .map(|other| other.name.clone())
                        .collect();
                    crate::names::fresh_apart(&mut names, &base)
                }
                None => crate::names::fresh_in(taken.entry(info.module).or_default(), &base),
            }
        };
        JsGiven::of_crate(&recognition, all_bodies, (&item_set, &unboxed), &copyable, &mut name)
    };
    let given = crate::lower::traits::Given {
        copied: &copied,
        clones: &clones,
        stepped: &stepped,
        unboxed: &unboxed,
        js_given: &js_given,
    };
    // Whether a collect's items drop, as the lowering's drops answer it
    // where they're held, given the drops their function is (#737).
    let items_drop = |typing_env: ty::TypingEnv<'tcx>, ty: Ty<'tcx>, owner: DefId, taken: &[u32]| {
        let recognition = Recognition {
            typing_env,
            ..recognition
        };
        super::drops::drops_given(
            recognition,
            (library, &closure_facts),
            &counted,
            (&foreign, owner, taken),
            ty,
        )
    };
    let item_drops = type_facts::ItemDrops {
        drops: &items_drop,
        taken: &drop_params,
    };
    let kept = (item_drops, asks_capacity || foreign.keeps_capacity());
    let facts = type_fact_params(
        tcx,
        all_bodies,
        &fns,
        (&foreign, library),
        given,
        (&recognition, &counted),
        kept,
    );
    let (failing, writes_may_fail, dictionaries_fail) = fmt_failures::failing_fns(tcx, all_bodies, &foreign, library);
    let throwing = throwing::throwing_fns(tcx, &recognition, all_bodies);
    let (never_unwinding, quiet_fns) = unwinding::found(tcx, &recognition, all_bodies, &runs);
    let caller_locations = caller_locations::caller_location_fns(tcx, all_bodies, &foreign);
    // A scope struct whose methods are given nothing besides their
    // arguments, no dictionary, fact, drop or location, as a function
    // declared in another is given none (#500).
    scopes.retain(|s| {
        s.methods.iter().all(|&m| {
            crate::lower::traits::bounds(tcx, &foreign, given, m).is_empty()
                && drop_params.get(&m).is_none_or(Vec::is_empty)
                && !facts.params.contains_key(&m)
                && !caller_locations.contains(&m)
        })
    });
    // Generic code's `extend` asking the crate's own hint, refused at its caller where none's kept (#849).
    crate::lower::iterators::refuse_unknown_capacity_extends(&recognition, all_bodies, kept.1);
    let generic_consts = generic_consts(tcx, all_bodies);
    // A program its tests use whose dictionaries lack a constant (#597).
    programs::refuse_lacked_consts(tcx, &foreign, &generic_consts);

    Some(AnalyzedCrate {
        bodies,
        flattened,
        closures,
        trait_impls,
        dictionaries,
        import_names,
        imported,
        required,
        required_uses,
        thread_local_inits,
        consts,
        variants,
        codecs,
        modules,
        taken,
        fns,
        failed,
        called_from_elsewhere,
        tests,
        paths,
        mutated,
        counted,
        self_handles,
        scopes,
        cell_borrows,
        wholes,
        closure_facts,
        cloned,
        explicit_adapters,
        back_stepped,
        hinted: hint_asked.hinted,
        chars_whole,
        changed_vecs,
        stored_places,
        drop_params,
        copied,
        clones,
        unboxed,
        js_given,
        stepped,
        type_facts: facts.params,
        carried_super: facts.carried_super,
        built_slots: facts.built_slots,
        failing,
        writes_may_fail,
        dictionaries_fail,
        throwing,
        never_unwinding,
        quiet_fns,
        caller_locations,
        asks_capacity,
        generic_consts,
        writer_facts,
        shown_params,
        reads_fn_identity,
        quiet_locals,
        quiet_library_keys,
        lazy_locals,
        locals_at_load,
        named_expressions,
        local_functions,
        local_consts,
        held,
        module_paths,
        null_fields,
    })
}

/// Whether `item` is a `pub use` its module's JS re-exports (ADR 0240): of
/// a function, the crate's, a library's or a binding's, or of a binding's
/// `static`; and of the crate's module, its namespace, or a class (ADR 0240,
/// amended).
pub(super) fn is_reexport(tcx: TyCtxt<'_>, item: &rustc_hir::Item<'_>) -> bool {
    let rustc_hir::ItemKind::Use(path, rustc_hir::UseKind::Single(_)) = item.kind else {
        return false;
    };
    tcx.visibility(item.owner_id).is_public()
        && (match path.res.value_ns {
            // A function's, and a constant's, its module's `const` (ADR 0596).
            Some(rustc_hir::def::Res::Def(DefKind::Fn | DefKind::Const { .. }, _)) => true,
            Some(rustc_hir::def::Res::Def(DefKind::Static { .. }, id)) => is_binding(tcx, id),
            _ => false,
        } || match path.res.type_ns {
            Some(rustc_hir::def::Res::Def(DefKind::Mod, id)) => id.is_local(),
            Some(rustc_hir::def::Res::Def(DefKind::Struct, id)) => crate::lower::classes::is_class(tcx, id),
            _ => false,
        })
}
