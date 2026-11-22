//! The immutable analysis inputs shared by function lowering, what the
//! pipeline found of the crate before any function is lowered: split from
//! `lower.rs` by concept.

use super::*;

/// Immutable analysis inputs shared by function lowering.
pub struct CrateFacts<'a, 'tcx> {
    pub sources: &'a sources::CapturedSources,
    /// The enums its classes hold as a flattened state, a `&mut` to each a
    /// handle (#478), and the enums told by their keys (#504):
    /// `bindings::flattened`.
    pub flattened: &'a bindings::Flattened,
    pub mutated: &'a super::analysis::Mutated<'tcx>,
    /// The `Rc`s counted (ADR 0320).
    pub counted: &'a analysis::Counted<'tcx>,
    /// Each closure's handle on itself that's no JS (ADR 0843, amended).
    pub self_handles: &'a analysis::SelfHandles<'tcx>,
    /// Each function's scope struct, whose methods are functions declared
    /// in it (ADR 1043).
    pub scopes: &'a analysis::Scopes<'tcx>,
    /// What each body may borrow, of its cells (ADR 0580).
    pub cell_borrows: &'a analysis::CellBorrows<'tcx>,
    /// The functions whose result is whole, and the fields each write of
    /// which is (rd537, ADR 0994, amended).
    pub wholes: &'a analysis::Wholes,
    /// Whether a function holding a value with a destructor is made a `dyn
    /// Fn` (ADR 0521).
    pub closure_facts: &'a analysis::ClosureFacts<'tcx>,
    /// The types the crate clones (ADR 0525).
    pub cloned: &'a analysis::Cloned<'tcx>,
    /// The crate's iterator facts: the types it steps from their back (ADR
    /// 0545), and the `take(n)`s, `skip(n)`s and `step_by(n)`s generic code
    /// asks the hint of, made `$taking`, `$skipping` or `$stepBy` (#1236).
    pub iterators: &'a analysis::IteratorFacts<'tcx>,
    /// Whether the crate asks a `Chars` its rest, its `Debug` or a `dyn` of
    /// it (#1186).
    pub chars_whole: bool,
    /// Whether the crate asks a `str`'s search by a pattern that runs code
    /// its back, its `Debug` or a `dyn` of it (#1501).
    pub searches_whole: bool,
    /// The trait constants generic code reads (ADR 0106).
    pub generic_consts: &'a IndexSet<DefId>,
    /// The functions JS gives again what it gave them (#1723).
    pub given_again: &'a IndexSet<DefId>,
    pub changed_vecs: &'a IndexSet<Ty<'tcx>>,
    /// The types whose objects say where they're stored (#1113).
    pub stored_places: &'a analysis::StoredPlaces<'tcx>,
    /// Each generic function's type parameters it's given a drop for (ADR 0098).
    pub drop_params: &'a IndexMap<DefId, Vec<u32>>,
    /// The type parameters whose `Copy` bound takes a copy function (ADR
    /// 0289), and whose `Iterator` its dictionary (ADR 0411).
    pub given: traits::Given<'a>,
    /// Each generic function's type parameters it's given a fact of (ADR 0145).
    pub type_facts: &'a IndexMap<DefId, Vec<(u32, TypeFact)>>,
    /// What each of the crate's traits' dictionaries carry of its std
    /// supertraits' associated types, which its defaults ask (#990).
    pub carried_super: &'a analysis::type_facts::CarriedSuper,
    /// The types built of each one's slots it's given a layout of (#737).
    pub built_slots: &'a traits::BuiltSlots<'tcx>,
    /// The functions that take their caller's location (ADR 0427).
    pub caller_locations: &'a IndexSet<DefId>,
    /// The functions that may return `Err(fmt::Error)`, whether any of the
    /// crate's or its libraries' may, and a dictionary's (ADR 0187, #1392).
    pub failing: analysis::Failing<'a>,
    /// The throwing functions, whose result is a throwing JS call's, each
    /// return its call's value (ADR 0644).
    pub throwing: &'a IndexSet<DefId>,
    /// The functions that never unwind, whose call throws only where what
    /// it's given does (ADR 0902).
    pub never_unwinding: &'a IndexSet<DefId>,
    /// The functions and closures whose calls can't be seen (#925).
    pub quiet_fns: &'a IndexSet<DefId>,
    /// The functions that read `js::arguments()`, each call of which gives
    /// every argument the Rust does (rd332).
    pub reads_arguments: &'a analysis::arguments::ReadsArguments<'tcx>,
    /// Whether a `Vec`'s capacity is kept, as the crate or a library it uses
    /// asks it (ADR 0419).
    pub keeps_capacity: bool,
    /// The functions that take only the drops they use (ADR 0300).
    pub takes_used_drops: &'a drops::TakesUsed,
    pub closures: &'a IndexMap<DefId, &'a Body<'tcx>>,
    pub bodies: &'a IndexMap<DefId, &'a Body<'tcx>>,
    pub fns: &'a IndexMap<DefId, FnInfo>,
    pub imports: &'a IndexMap<LocalModId, IndexMap<Export, String>>,
    /// The exports read through `require(module)`, not imported (ADR 0305).
    pub required: &'a IndexSet<Export>,
    /// Each library's function a module's `use` reads through
    /// `require(module)`, at each read, not imported (rd684, ADR 0305,
    /// amended).
    pub required_uses: &'a IndexSet<(LocalModId, DefId)>,
    /// Where a path reads a binding through the module that holds it, by
    /// the path's span (ADR 0633).
    pub held_reads: &'a IndexMap<Span, bindings::HeldRead>,
    /// Where a path names a function through the module that has it, by
    /// the path's span and its call's: read through that module's
    /// namespace, `Plugin.extractEvents(..)` (rd114, ADR 1018).
    pub module_paths: &'a IndexMap<Span, analysis::Through>,
    /// What the crate's libraries export (ADR 0100).
    pub foreign: &'a library::Foreign<'a, 'tcx>,
    /// Whether the crate asked its panics to abort, `-C panic=abort`, which
    /// rust-js unwinds (ADR 0090): unwinding is refused of it (ADR 0660).
    pub panics_abort: bool,
    /// Is this crate compiled as a library, for others to use (ADR 0100)?
    pub library: bool,
    /// Is it `@rust-js/std`, compiled once into a package for every
    /// consumer, which none compiles again (ADR 0561)? Another library's
    /// JS is its program's, and a consumer compiles what it needs more of
    /// (ADR 0914, amended).
    pub compiled_once: bool,
    /// Which of its libraries' functions and impls may copy what the crate
    /// changes in place, which it compiles itself (ADR 0914, amended).
    pub copying: &'a instances::Copying<'a, 'tcx>,
    pub trait_impls: &'a [DefId],
    /// `#[serde(..)]` attributes, from the expanded crate (ADR 0077).
    pub serde_attrs: &'a serde::SerdeAttributes,
    /// Do its `Debug` functions take whether to be pretty (ADR 0137)?
    pub pretty_debug: bool,
    /// Does it give a placeholder's options to its writers and dictionaries
    /// (ADR 0058)?
    pub format_options: bool,
    /// Whether the crate asks `{:x?}` of anything: then an integer a writer
    /// shows is in hex where its options say (ADR 0559).
    pub debug_hex: bool,
    /// What's shown with options of each type it gives a parameter shown
    /// so, pretty or in hex, whose writers then take them (#1331).
    pub shown: &'a analysis::ShownParams<'tcx>,
    /// Which types' formatting dictionaries carry `fmtInto`, as a writer's
    /// generic code or a `dyn` writes them into a writer (#1652).
    pub written: &'a analysis::WrittenParams<'tcx>,
    /// Which writers print as they format, and what may run while one
    /// does (#1490).
    pub prints: &'a analysis::prints::Prints,
    /// The pass tidy stops after, `--tidy-until`'s (ADR 0659).
    pub tidy_until: Option<&'a str>,
    /// The `RefCell`s a field holds as their value, never counted (ADR
    /// 0362), and those whose borrows are counted (ADR 0863): a `Lazy`, as
    /// finding them is about 2% of a compile, found the first time a body
    /// asks.
    pub ref_cells: &'a Lazy<drops::RefCells<'tcx>>,
    /// The parameters of `dyn Fn` each of the crate's calls fills with a
    /// function that reads no `this` and none of what it's given more,
    /// found before any body (`contexts::own_callbacks`, rd616): under 0.1%
    /// of a compile.
    pub own_callbacks: &'a IndexSet<rustc_hir::HirId>,
    /// The thread-locals each `with_borrow` of which can't ask whether it's
    /// borrowed, their module's variables (ADRs 0270, 0580).
    pub quiet_locals: &'a IndexSet<rustc_span::def_id::LocalDefId>,
    /// A library's thread-locals none of whose borrows here can meet
    /// another (`plain_locals::quiet_library_keys`, ADR 0863, amended).
    pub quiet_library_keys: &'a IndexSet<DefId>,
    /// The thread-locals made where they're first used, read by calling
    /// them (ADR 0037, amended).
    pub lazy_locals: &'a IndexSet<rustc_span::def_id::LocalDefId>,
    /// All eager keys from lazy_locals, including constant initializers.
    pub eager_locals: &'a IndexSet<rustc_span::def_id::LocalDefId>,
    /// The functions a block makes and gives, each a named function
    /// expression there (ADR 0296).
    pub named_expressions: &'a IndexSet<DefId>,
    /// The functions written in a function's body, by the block they're
    /// in, each a function declaration there (ADR 0308).
    pub local_functions: &'a IndexMap<DefId, rustc_span::Span>,
    /// Each namespace's method that's the function it passes its
    /// parameters on to, by that function (rd292b, ADR 1037 amended).
    pub forwarded: &'a IndexMap<DefId, DefId>,
    /// The `const`s written in a function's body, each a `const` there,
    /// by the block they're in and the bodies that read them (ADR 0031,
    /// amended).
    pub local_consts: &'a analysis::LocalConsts,
    /// How many literals the crate writes of each struct: one's fields'
    /// doc comments are its object's (ADR 0444, amended).
    pub struct_literals: &'a IndexMap<DefId, usize>,
    /// The nullable fields present, set first (`classes::set_where_constructed`).
    pub constructed: &'a IndexSet<DefId>,
    /// The own nullable parameters each call gives `null` or a value, as
    /// lowering sees their calls (ADR 0861, amended, rd613).
    pub nulled_params: &'a analysis::nulled_params::NulledParams,
}

impl CrateFacts<'_, '_> {
    /// Whether the crate's item `def_id` is declared at its module's top,
    /// so its name is the module's (`js::Name::module`): not a function or
    /// a `const` written in a function's body, nor a function a block makes
    /// and gives, each declared where it's written (ADRs 0296, 0308, 0031
    /// amended), whose name is its body's ("names are ids", ADR 1150).
    fn at_module_top(&self, def_id: DefId) -> bool {
        !self.local_functions.contains_key(&def_id)
            && !self.named_expressions.contains(&def_id)
            && !self.local_consts.contains_key(&def_id)
    }

    /// The name of the crate's item `def_id`, or of its copy `copy`, spelled
    /// `word`: the module's at its top, else its body's, the item's own
    /// (`js::Name::item`), whichever lowering asks, its declaration's or a
    /// read's ("names are ids", ADR 1150).
    pub fn item_name(&self, def_id: DefId, copy: Option<usize>, word: &str) -> rust_js_program::js::Name {
        match self.at_module_top(def_id) {
            true => rust_js_program::js::Name::module(word),
            false => rust_js_program::js::Name::item(word, def_id.index.as_u32(), copy),
        }
    }

    /// Do its writers take their `Formatter`'s options after their
    /// arguments, whether to be pretty or a placeholder's (ADRs 0058, 0137)?
    pub fn writers_take_options(&self) -> bool {
        self.pretty_debug || self.format_options
    }
}

/// A fact of the crate before any body that's found only the first time a
/// body asks, as finding it eagerly costs more than 1% of a compile (ADR
/// 1148): found of the crate's facts alone, never of what a body found, so
/// it's the same whichever body asks first. The one memo `CrateFacts` may
/// hold; test/architecture-observations.test.ts holds who finds each.
pub struct Lazy<T>(std::cell::OnceCell<T>);

impl<T> Default for Lazy<T> {
    fn default() -> Self {
        Lazy(std::cell::OnceCell::new())
    }
}

impl<T> Lazy<T> {
    /// It, found by `find` the first time it's asked.
    pub(in crate::lower) fn get_or_find(&self, find: impl FnOnce() -> T) -> &T {
        self.0.get_or_init(find)
    }

    /// It, where a body asked it.
    pub fn get(&self) -> Option<&T> {
        self.0.get()
    }
}

/// What lowering the crate's bodies settled so far, which a body lowered
/// after reads: the pipeline's, between one body and the next, never a
/// body's own (#1671 change 4, M2). A body reads it, and gives back what it
/// found in its `LoweredFn`, which the pipeline settles here.
#[derive(Default)]
pub struct Settled<'tcx> {
    /// Each method that reads each variant's field, its JS's parameter and
    /// read, of one lowered (rd317): lowered before its callers.
    pub field_reads: IndexMap<DefId, (js::Name, Expr)>,
    /// Each body run where its module loads, a thread-local's value, a
    /// static's or constant's initializer lowered as code, a `js::on_load!`,
    /// that reads another of the crate's modules' `const` that may load
    /// after its own, an import cycle's: it reads their values
    /// (`FnCx::reads_by_name`, `pipeline::load_cycles`, ADR 0031, amended).
    /// Lowered again once it's known.
    pub loads_in_cycles: IndexSet<DefId>,
    /// Each module's function of a recursive type's `==`, clone or order,
    /// and its dictionaries' and instances' values, made by the bodies
    /// lowered before (`recursive_fns`): each body's own given back.
    pub module_fns: recursive_fns::ModuleFns<'tcx>,
    /// What the bodies lowered before found of the own nullable parameters
    /// each call gives `null` or a value (ADR 0861, amended, rd613): each
    /// body's `NulledSeen` settled.
    pub nulled: analysis::nulled_params::NulledFound,
}

/// Whether a library's consumer needs a crate fact (#1671 change 4 M4):
/// none of what the crate alone reads, or the contract's facts that carry
/// it across, each with its scope (`rust_js_program::library::scopes`).
#[cfg(test)]
enum Carried {
    CrateOnly,
    By(&'static [rust_js_program::library::scopes::ContractField]),
}

/// Each of the crate's facts, by whether a consumer needs it: a pattern
/// naming every field of `CrateFacts`, so one it gains is rustc's error in
/// `cargo test` until it says, and the list. Adding a fact asks whether a
/// library's consumer needs it, which #1772's fact had no one decide.
#[cfg(test)]
macro_rules! carried {
    ($($field:ident => $carried:expr,)*) => {
        impl CrateFacts<'_, '_> {
            fn named(&self) {
                let CrateFacts { $($field),* } = self;
                $(let _ = $field;)*
            }
        }

        const CARRIED: &[(&str, Carried)] = &[$((stringify!($field), $carried)),*];
    };
}

#[cfg(test)]
use {Carried::*, rust_js_program::library::scopes::ContractField as F};

#[cfg(test)]
carried! {
    sources => CrateOnly,
    flattened => By(&[F::LibraryKeyed]),
    mutated => By(&[F::LibraryChanged]),
    counted => By(&[F::LibraryCountedRcs, F::ParamFactCounted]),
    self_handles => By(&[F::CallingOwnedByReturn, F::CallingQuietByReturn]),
    scopes => CrateOnly,
    cell_borrows => By(&[F::LibraryBuilt, F::RefCellFactsBoxed, F::RefCellFactsCounted, F::RefCellFactsUnchecked, F::RefCellFactsHanded, F::FieldFactRustFunctions]),
    wholes => CrateOnly,
    closure_facts => By(&[F::LibraryMakesDroppingAnys, F::LibraryMakesDroppingErrors, F::LibraryMakesDroppingFnDyns, F::LibraryMakesDroppingIterators, F::LibraryClonedClosures]),
    cloned => CrateOnly,
    iterators => CrateOnly,
    chars_whole => CrateOnly,
    searches_whole => CrateOnly,
    generic_consts => By(&[F::LibraryDictionaryConsts]),
    given_again => CrateOnly,
    changed_vecs => By(&[F::LibraryChanged]),
    stored_places => CrateOnly,
    drop_params => By(&[F::ParamFactDrop]),
    given => By(&[F::ParamFactCopy, F::ParamFactClone, F::ParamFactUnboxed, F::TypeParamStepped, F::TypeParamUnread]),
    type_facts => By(&[F::TypeParamFacts]),
    carried_super => By(&[F::LibraryAssociated]),
    built_slots => By(&[F::TypeParamBuilt]),
    caller_locations => By(&[F::ItemFactTakesLocation]),
    failing => By(&[F::ItemFactFails, F::LibraryWritesMayFail]),
    throwing => CrateOnly,
    never_unwinding => CrateOnly,
    quiet_fns => CrateOnly,
    reads_arguments => By(&[F::ItemFactArguments, F::ItemFactThis, F::LibraryWithThisArguments]),
    keeps_capacity => By(&[F::LibraryKeepsCapacity]),
    takes_used_drops => CrateOnly,
    closures => CrateOnly,
    bodies => CrateOnly,
    fns => CrateOnly,
    imports => CrateOnly,
    required => CrateOnly,
    required_uses => CrateOnly,
    held_reads => CrateOnly,
    module_paths => CrateOnly,
    foreign => CrateOnly,
    panics_abort => CrateOnly,
    library => CrateOnly,
    compiled_once => CrateOnly,
    copying => CrateOnly,
    trait_impls => By(&[F::LibraryImpls]),
    serde_attrs => CrateOnly,
    pretty_debug => By(&[F::LibraryWritersTakeOptions]),
    format_options => By(&[F::LibraryWritersTakeOptions]),
    debug_hex => By(&[F::LibraryDebugHex]),
    shown => By(&[F::ParamFactShownPretty, F::ParamFactShownHex, F::LibraryDynDebugPretty, F::LibraryDynDebugHex]),
    written => By(&[F::ParamFactWrittenInto, F::CallingWrittenArguments, F::LibraryWrittenInto, F::LibraryWrittenTraitArguments, F::LibraryDynWrittenInto, F::LibraryMakesFmtDyns]),
    prints => By(&[F::LibraryPrintingTypes, F::LibraryStoppingTypes, F::LibraryStoppingWhereCaught, F::LibraryLeavesUnended, F::LibraryFramedWriters]),
    tidy_until => CrateOnly,
    ref_cells => By(&[F::RefCellFactsBoxed, F::RefCellFactsCounted, F::RefCellFactsUnchecked, F::RefCellFactsHanded, F::FieldFactQuiet]),
    own_callbacks => CrateOnly,
    quiet_locals => CrateOnly,
    quiet_library_keys => CrateOnly,
    lazy_locals => By(&[F::ItemFactLazy]),
    eager_locals => CrateOnly,
    named_expressions => CrateOnly,
    local_functions => CrateOnly,
    forwarded => CrateOnly,
    local_consts => CrateOnly,
    struct_literals => CrateOnly,
    constructed => CrateOnly,
    nulled_params => By(&[F::CallingNulledParams]),
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Each crate fact says whether a consumer needs it, by the facts of
    /// the contract that carry it, none a list of rows alone.
    #[test]
    fn each_crate_fact_says_whether_a_consumer_needs_it() {
        let _ = CrateFacts::named;
        for (name, carried) in CARRIED {
            if let By(fields) = carried {
                assert!(!fields.is_empty(), "{name} is carried by nothing");
                for field in *fields {
                    let (fact, scope, _) = field.row();
                    assert!(
                        scope != rust_js_program::library::scopes::FactScope::Holds,
                        "{name} is carried by {fact}, a list"
                    );
                }
            }
        }
        assert_eq!(CARRIED.len(), 65);
    }
}
