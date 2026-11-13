//! The immutable analysis inputs shared by function lowering, what the
//! pipeline found of the crate before any function is lowered: split from
//! `lower.rs` by concept.

use super::*;

/// Immutable analysis inputs shared by function lowering.
pub(in crate::lower) struct CrateFacts<'a, 'tcx> {
    pub(in crate::lower) sources: &'a sources::CapturedSources,
    /// The enums its classes hold as a flattened state, a `&mut` to each a
    /// handle (#478), and the enums told by their keys (#504):
    /// `bindings::flattened`.
    pub(in crate::lower) flattened: &'a bindings::Flattened,
    pub(in crate::lower) mutated: &'a IndexSet<Ty<'tcx>>,
    /// The `Rc`s counted (ADR 0320).
    pub(in crate::lower) counted: &'a analysis::Counted<'tcx>,
    /// Each closure's handle on itself that's no JS (ADR 0843, amended).
    pub(in crate::lower) self_handles: &'a analysis::SelfHandles<'tcx>,
    /// Each function's scope struct, whose methods are functions declared
    /// in it (ADR 1043).
    pub(in crate::lower) scopes: &'a analysis::Scopes<'tcx>,
    /// What each body may borrow, of its cells (ADR 0580).
    pub(in crate::lower) cell_borrows: &'a analysis::CellBorrows<'tcx>,
    /// The functions whose result is whole, and the fields each write of
    /// which is (rd537, ADR 0994, amended).
    pub(in crate::lower) wholes: &'a analysis::Wholes,
    /// Whether a function holding a value with a destructor is made a `dyn
    /// Fn` (ADR 0521).
    pub(in crate::lower) closure_facts: &'a analysis::ClosureFacts<'tcx>,
    /// The types the crate clones (ADR 0525).
    pub(in crate::lower) cloned: &'a analysis::Cloned<'tcx>,
    /// The kinds of std's adapters explicit wherever the crate makes one
    /// (ADR 0542): those its types hold, and those with a count it clones.
    pub(in crate::lower) explicit_adapters: &'a IndexSet<DefId>,
    /// The map and filter types the crate steps from their back (ADR 0545).
    pub(in crate::lower) back_stepped: &'a analysis::BackStepped<'tcx>,
    /// The `take(n)`s, `skip(n)`s and `step_by(n)`s generic code asks the
    /// hint of, made `$taking`, `$skipping` or `$stepBy` (#1236).
    pub(in crate::lower) hinted: &'a analysis::Hinted<'tcx>,
    /// Whether the crate asks a `Chars` its rest, its `Debug` or a `dyn` of
    /// it (#1186).
    pub(in crate::lower) chars_whole: bool,
    /// Whether the crate asks a `str`'s search by a pattern that runs code
    /// its back, its `Debug` or a `dyn` of it (#1501).
    pub(in crate::lower) searches_whole: bool,
    /// The trait constants generic code reads (ADR 0106).
    pub(in crate::lower) generic_consts: &'a IndexSet<DefId>,
    pub(in crate::lower) changed_vecs: &'a IndexSet<Ty<'tcx>>,
    /// The types whose objects say where they're stored (#1113).
    pub(in crate::lower) stored_places: &'a analysis::StoredPlaces<'tcx>,
    /// Each generic function's type parameters it's given a drop for (ADR 0098).
    pub(in crate::lower) drop_params: &'a IndexMap<DefId, Vec<u32>>,
    /// The type parameters whose `Copy` bound takes a copy function (ADR
    /// 0289), and whose `Iterator` its dictionary (ADR 0411).
    pub(in crate::lower) given: traits::Given<'a>,
    /// Each generic function's type parameters it's given a fact of (ADR 0145).
    pub(in crate::lower) type_facts: &'a IndexMap<DefId, Vec<(u32, TypeFact)>>,
    /// What each of the crate's traits' dictionaries carry of its std
    /// supertraits' associated types, which its defaults ask (#990).
    pub(in crate::lower) carried_super: &'a analysis::type_facts::CarriedSuper,
    /// The types built of each one's slots it's given a layout of (#737).
    pub(in crate::lower) built_slots: &'a traits::BuiltSlots<'tcx>,
    /// The functions that take their caller's location (ADR 0427).
    pub(in crate::lower) caller_locations: &'a IndexSet<DefId>,
    /// The functions that may return `Err(fmt::Error)`, whether any of the
    /// crate's or its libraries' may, and a dictionary's (ADR 0187, #1392).
    pub(in crate::lower) failing: analysis::Failing<'a>,
    /// The throwing functions, whose result is a throwing JS call's, each
    /// return its call's value (ADR 0644).
    pub(in crate::lower) throwing: &'a IndexSet<DefId>,
    /// The functions that never unwind, whose call throws only where what
    /// it's given does (ADR 0902).
    pub(in crate::lower) never_unwinding: &'a IndexSet<DefId>,
    /// The functions and closures whose calls can't be seen (#925).
    pub(in crate::lower) quiet_fns: &'a IndexSet<DefId>,
    /// The functions that read `js::arguments()`, each call of which gives
    /// every argument the Rust does (rd332).
    pub(in crate::lower) reads_arguments: &'a analysis::arguments::ReadsArguments<'tcx>,
    /// Whether a `Vec`'s capacity is kept, as the crate or a library it uses
    /// asks it (ADR 0419).
    pub(in crate::lower) keeps_capacity: bool,
    /// The functions that take only the drops they use (ADR 0300).
    pub(in crate::lower) takes_used_drops: &'a drops::TakesUsed,
    pub(in crate::lower) closures: &'a IndexMap<DefId, &'a Body<'tcx>>,
    pub(in crate::lower) bodies: &'a IndexMap<DefId, &'a Body<'tcx>>,
    pub(in crate::lower) fns: &'a IndexMap<DefId, FnInfo>,
    pub(in crate::lower) imports: &'a IndexMap<LocalModId, IndexMap<Export, String>>,
    /// The exports read through `require(module)`, not imported (ADR 0305).
    pub(in crate::lower) required: &'a IndexSet<Export>,
    /// Each library's function a module's `use` reads through
    /// `require(module)`, at each read, not imported (rd684, ADR 0305,
    /// amended).
    pub(in crate::lower) required_uses: &'a IndexSet<(LocalModId, DefId)>,
    /// Where a path reads a binding through the module that holds it, by
    /// the path's span (ADR 0633).
    pub(in crate::lower) held_reads: &'a IndexMap<Span, bindings::HeldRead>,
    /// Where a path names a function through the module that has it, by
    /// the path's span and its call's: read through that module's
    /// namespace, `Plugin.extractEvents(..)` (rd114, ADR 1018).
    pub(in crate::lower) module_paths: &'a IndexMap<Span, analysis::Through>,
    /// What the crate's libraries export (ADR 0100).
    pub(in crate::lower) foreign: &'a library::Foreign<'a, 'tcx>,
    /// Whether the crate asked its panics to abort, `-C panic=abort`, which
    /// rust-js unwinds (ADR 0090): unwinding is refused of it (ADR 0660).
    pub(in crate::lower) panics_abort: bool,
    /// Is this crate compiled as a library, for others to use (ADR 0100)?
    pub(in crate::lower) library: bool,
    /// Is it `@rust-js/std`, compiled once into a package for every
    /// consumer, which none compiles again (ADR 0561)? Another library's
    /// JS is its program's, and a consumer compiles what it needs more of
    /// (ADR 0914, amended).
    pub(in crate::lower) compiled_once: bool,
    /// The library items the crate's lowering found it must compile itself,
    /// which their libraries' JS doesn't export, each with the module that
    /// first asked (ADR 0914, amended): the crate is lowered again with them.
    pub(in crate::lower) asked: std::cell::RefCell<Vec<(DefId, LocalModId)>>,
    /// Which of its libraries' functions and impls may copy what the crate
    /// changes in place, which it compiles itself (ADR 0914, amended).
    pub(in crate::lower) copying: &'a instances::Copying<'a, 'tcx>,
    pub(in crate::lower) trait_impls: &'a [DefId],
    /// `#[serde(..)]` attributes, from the expanded crate (ADR 0077).
    pub(in crate::lower) serde_attrs: &'a serde::SerdeAttributes,
    /// Do its `Debug` functions take whether to be pretty (ADR 0137)?
    pub(in crate::lower) pretty_debug: bool,
    /// Does it give a placeholder's options to its writers and dictionaries
    /// (ADR 0058)?
    pub(in crate::lower) format_options: bool,
    /// Whether the crate asks `{:x?}` of anything: then an integer a writer
    /// shows is in hex where its options say (ADR 0559).
    pub(in crate::lower) debug_hex: bool,
    /// What's shown with options of each type it gives a parameter shown
    /// so, pretty or in hex, whose writers then take them (#1331).
    pub(in crate::lower) shown: &'a analysis::ShownParams<'tcx>,
    /// Which types' formatting dictionaries carry `fmtInto`, as a writer's
    /// generic code or a `dyn` writes them into a writer (#1652).
    pub(in crate::lower) written: &'a analysis::WrittenParams<'tcx>,
    /// Which writers print as they format, and what may run while one
    /// does (#1490).
    pub(in crate::lower) prints: &'a analysis::prints::Prints,
    /// The pass tidy stops after, `--tidy-until`'s (ADR 0659).
    pub(in crate::lower) tidy_until: Option<&'a str>,
    /// The `RefCell`s a field holds as their value, never counted (ADR
    /// 0362), and those whose borrows are counted (ADR 0863), found once
    /// the first is asked of.
    pub(in crate::lower) ref_cells: &'a std::cell::OnceCell<drops::RefCells<'tcx>>,
    /// The parameters of `dyn Fn` each of the crate's calls fills with a
    /// function that reads no `this` and none of what it's given more,
    /// found once the first is asked of (`passed_on`'s `own_callbacks`,
    /// rd616).
    pub(in crate::lower) own_callbacks: &'a std::cell::OnceCell<IndexSet<rustc_hir::HirId>>,
    /// The thread-locals each `with_borrow` of which can't ask whether it's
    /// borrowed, their module's variables (ADRs 0270, 0580).
    pub(in crate::lower) quiet_locals: &'a IndexSet<rustc_span::def_id::LocalDefId>,
    /// A library's thread-locals none of whose borrows here can meet
    /// another (`plain_locals::quiet_library_keys`, ADR 0863, amended).
    pub(in crate::lower) quiet_library_keys: &'a IndexSet<DefId>,
    /// The thread-locals made where they're first used, read by calling
    /// them (ADR 0037, amended).
    pub(in crate::lower) lazy_locals: &'a IndexSet<rustc_span::def_id::LocalDefId>,
    /// The functions a block makes and gives, each a named function
    /// expression there (ADR 0296).
    pub(in crate::lower) named_expressions: &'a IndexSet<DefId>,
    /// The functions written in a function's body, by the block they're
    /// in, each a function declaration there (ADR 0308).
    pub(in crate::lower) local_functions: &'a IndexMap<DefId, rustc_span::Span>,
    /// Each namespace's method that's the function it passes its
    /// parameters on to, by that function (rd292b, ADR 1037 amended).
    pub(in crate::lower) forwarded: &'a IndexMap<DefId, DefId>,
    /// The `const`s written in a function's body, each a `const` there,
    /// by the block they're in and the bodies that read them (ADR 0031,
    /// amended).
    pub(in crate::lower) local_consts: &'a analysis::LocalConsts,
    /// How many literals the crate writes of each struct: one's fields'
    /// doc comments are its object's (ADR 0444, amended).
    pub(in crate::lower) struct_literals: &'a IndexMap<DefId, usize>,
    /// The nullable fields present, set first (`classes::set_where_constructed`).
    pub(in crate::lower) constructed: &'a IndexSet<DefId>,
    /// Each body run where its module loads, a thread-local's value, a
    /// static's or constant's initializer lowered as code, a `js::on_load!`,
    /// that reads another of the crate's modules' `const` that may load
    /// after its own, an import cycle's: it reads their values
    /// (`FnCx::reads_by_name`, `pipeline::load_cycles`, ADR 0031, amended).
    /// Lowered again once it's known.
    pub(in crate::lower) loads_in_cycles: std::cell::RefCell<IndexSet<DefId>>,
    /// Each module's function of a recursive type's `==`, clone or order.
    pub(in crate::lower) module_fns: RefCell<recursive_fns::ModuleFns<'tcx>>,
    /// Each method that reads each variant's field, its JS's parameter and read (rd317).
    pub(in crate::lower) field_reads: RefCell<IndexMap<DefId, (String, Expr)>>,
    /// The own nullable parameters each call gives `null` or a value, as
    /// lowering sees their calls (ADR 0861, amended, rd613).
    pub(in crate::lower) nulled_params: &'a analysis::nulled_params::NulledParams,
}

impl CrateFacts<'_, '_> {
    /// Do its writers take their `Formatter`'s options after their
    /// arguments, whether to be pretty or a placeholder's (ADRs 0058, 0137)?
    pub(in crate::lower) fn writers_take_options(&self) -> bool {
        self.pretty_debug || self.format_options
    }
}
