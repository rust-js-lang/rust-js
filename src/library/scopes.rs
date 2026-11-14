//! What makes each fact of a library's contract true, and what a consumer
//! that differs does (ADR 1129, amended; #1671 change 4 M4): a
//! `ContractField` of each field of `Library`, `Calling`, `TypeParam` and
//! `RefCellFacts` and of each variant of `ItemFact`, `ParamFact` and
//! `FieldFact`, and its `FactScope`, an exhaustive match. A field or a
//! variant the contract gains is rustc's error in `cargo test` until it's
//! given a `ContractField`, and that one's until it's given its scope:
//! adding a fact asks whether a consumer needs it, and what one that sees
//! otherwise does. docs/library-contract.md's table is made of it
//! (`BLESS=1 cargo test contract_table`).

use super::{Calling, FieldFact, ItemFact, Library, ParamFact, RefCellFacts, TypeParam};

/// What makes a fact of the contract true.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum FactScope {
    /// What the library's source declares, whatever its bodies or its
    /// callers do: a consumer can't link without it.
    Local,
    /// Of one function, impl or item alone, and what it calls:
    /// compositional, as a signature is. A consumer gives it at its call,
    /// or is checked against it.
    Summary,
    /// Of what the library's whole crate does, or never does: an
    /// optimisation only sound while every use is seen, so said with what
    /// a consumer that sees otherwise does.
    ClosedWorld(Differs),
    /// A list of the rows named, no fact of its own.
    Holds,
}

/// What a consumer that sees otherwise than a closed-world fact says
/// does, and how, as its row says.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Differs {
    /// It reaches the same answer from its own view, or does what makes
    /// the library's answer hold of it.
    Agree(&'static str),
    /// It compiles its own of the library's function (ADR 0914).
    CompileOwn(&'static str),
    /// It's refused.
    Refuse(&'static str),
}

/// A fact of the contract: a field of one of its types, or a variant of
/// one of its sets of facts.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ContractField {
    LibraryVersion,
    LibraryName,
    LibraryCrateId,
    LibraryCrateHash,
    LibraryLibraries,
    LibraryInputs,
    LibraryPackage,
    LibraryItems,
    LibraryImpls,
    LibraryHeld,
    LibraryReexported,
    LibraryModules,
    LibraryTypes,
    LibraryKeyed,
    CallingParams,
    CallingFacts,
    TypeParamIndex,
    TypeParamMarks,
    TypeParamFacts,
    TypeParamStepped,
    TypeParamBuilt,
    ParamFactDrop,
    ParamFactCopy,
    ParamFactClone,
    ParamFactNoDrop,
    ParamFactShownPretty,
    ParamFactShownHex,
    ParamFactWrittenInto,
    ItemFactFails,
    ItemFactTakesLocation,
    ItemFactRest,
    ItemFactArguments,
    ItemFactThis,
    ItemFactLazy,
    ItemFactVariable,
    CallingOwnedByReturn,
    CallingQuietByReturn,
    CallingWrittenArguments,
    LibraryWithThisArguments,
    LibraryGivesArgument,
    LibraryDisplaysValue,
    LibraryPartialCmpIsCmp,
    LibraryIdentities,
    LibraryEqByAddress,
    LibraryAssociated,
    ParamFactUnboxed,
    ParamFactCounted,
    CallingNulledParams,
    LibraryKeepsCapacity,
    LibraryWritersTakeOptions,
    LibraryDebugHex,
    LibraryDynDebugPretty,
    LibraryDynDebugHex,
    LibraryReadsFormatterOptions,
    LibraryWritesMayFail,
    LibraryMakesDroppingAnys,
    LibraryMakesDroppingErrors,
    LibraryMakesDroppingFnDyns,
    LibraryMakesDroppingIterators,
    LibraryDropsAny,
    LibraryPrintingTypes,
    LibraryStoppingTypes,
    LibraryStoppingWhereCaught,
    LibraryWrittenInto,
    LibraryWrittenTraitArguments,
    LibraryFramedWriters,
    LibraryDynWrittenInto,
    LibraryMakesFmtDyns,
    LibraryLeavesUnended,
    LibraryDictionaryConsts,
    LibraryDynsDropNothing,
    LibraryCountedRcs,
    LibraryChanged,
    LibraryMergesIdentities,
    LibraryBuilt,
    LibraryClonedClosures,
    LibraryRefCells,
    RefCellFactsBoxed,
    RefCellFactsCounted,
    RefCellFactsUnchecked,
    RefCellFactsHanded,
    LibraryFields,
    LibraryTraitMethods,
    FieldFactQuiet,
    FieldFactRustFunctions,
}

use ContractField::*;

impl ContractField {
    /// Its row: its name, as the contract's types have it, its scope, and
    /// what it says.
    pub(crate) fn row(self) -> (&'static str, FactScope, &'static str) {
        use Differs::*;
        use FactScope::*;
        match self {
            LibraryVersion => (
                "Library.version",
                Local,
                "the contract's own version; another is refused",
            ),
            LibraryName => ("Library.name", Local, "the crate's name"),
            LibraryCrateId => (
                "Library.crate_id",
                Local,
                "rustc's identity of the crate, with its name the key it's known by",
            ),
            LibraryCrateHash => (
                "Library.crate_hash",
                Local,
                "rustc's hash of the crate its JS was made from, checked against the metadata loaded",
            ),
            LibraryLibraries => (
                "Library.libraries",
                Local,
                "the libraries it was compiled against, whose manifests its consumers must have",
            ),
            LibraryInputs => ("Library.inputs", Local, "the files it was made of, each with its hash"),
            LibraryPackage => ("Library.package", Local, "its npm package, where its crate has one"),
            LibraryItems => (
                "Library.items",
                Local,
                "each exported item: its key, path, module, export and member, and what its calls give it (`Calling`)",
            ),
            LibraryImpls => ("Library.impls", Local, "the trait impls whose methods are items"),
            LibraryHeld => (
                "Library.held",
                Local,
                "the bindings its modules hold as values of their own",
            ),
            LibraryReexported => (
                "Library.reexported",
                Local,
                "what its modules re-export of its others' items",
            ),
            LibraryModules => (
                "Library.modules",
                Local,
                "its modules, each a namespace a consumer may re-export",
            ),
            LibraryTypes => ("Library.types", Local, "the types its declarations export by name"),
            LibraryKeyed => (
                "Library.keyed",
                Local,
                "its enums told apart by their keys, no tag, by their shape",
            ),
            CallingParams => (
                "Calling.params",
                Holds,
                "each type parameter it asks anything of: `TypeParam`",
            ),
            CallingFacts => ("Calling.facts", Holds, "`ItemFact`"),
            TypeParamIndex => ("TypeParam.index", Summary, "which type parameter"),
            TypeParamMarks => ("TypeParam.marks", Holds, "`ParamFact`"),
            TypeParamFacts => (
                "TypeParam.facts",
                Summary,
                "the type facts it asks of it, `Size`, `Align`, `RoomSize` and the rest (ADR 0145), which its call gives; of a built receiver, the offset of what std lends from inside it, `OkOffset`, `ErrOffset` and the like, true of the consumer's instance as `Size` is: a call that can't give it, of no layout, is refused (#1478)",
            ),
            TypeParamStepped => (
                "TypeParam.stepped",
                Summary,
                "the traits it's stepped by as it is, each given a dictionary of",
            ),
            TypeParamBuilt => (
                "TypeParam.built",
                Summary,
                "of a slot past its type parameters', the type built of them whose layout it asks, or where in it std lends from, `Result<T, E>`'s `Ok` (#1478)",
            ),
            ParamFactDrop => ("ParamFact::Drop", Summary, "it's given what drops a value of it"),
            ParamFactCopy => ("ParamFact::Copy", Summary, "it's given what copies a value of it"),
            ParamFactClone => ("ParamFact::Clone", Summary, "it's given its `Clone` dictionary"),
            ParamFactNoDrop => (
                "ParamFact::NoDrop",
                Summary,
                "it takes no destructor: a call that gives one is refused",
            ),
            ParamFactShownPretty => (
                "ParamFact::ShownPretty",
                Summary,
                "it's shown pretty, here or by what it's given to: a writer of a type given it takes whether",
            ),
            ParamFactShownHex => ("ParamFact::ShownHex", Summary, "it's shown in hex, as `ShownPretty`"),
            ParamFactWrittenInto => (
                "ParamFact::WrittenInto",
                Summary,
                "it's written into a writer a piece at a time through its formatting dictionaries, here or by what it's given to: a call gives each with its `fmtInto` (#1652)",
            ),
            ItemFactFails => ("ItemFact::Fails", Summary, "it may return `Err(fmt::Error)`"),
            ItemFactTakesLocation => (
                "ItemFact::TakesLocation",
                Local,
                "`#[track_caller]`: its calls give it their location",
            ),
            ItemFactRest => (
                "ItemFact::Rest",
                Local,
                "`#[rust_js::rest]`: its last parameter is JS's rest, which its calls spread",
            ),
            ItemFactArguments => (
                "ItemFact::Arguments",
                Summary,
                "it reads JS's `arguments`: its calls give every argument, and no closure that forwards to it is it",
            ),
            ItemFactThis => (
                "ItemFact::This",
                Summary,
                "it reads JS's `this`: its calls give it none, and it's used only to be called",
            ),
            ItemFactLazy => (
                "ItemFact::Lazy",
                Summary,
                "a thread-local made where it's first used, read by calling it",
            ),
            ItemFactVariable => ("ItemFact::Variable", Summary, "its module's variable, read by its name"),
            CallingOwnedByReturn => (
                "Calling.owned_by_return",
                Summary,
                "the parameters what it returns alone owns",
            ),
            CallingQuietByReturn => (
                "Calling.quiet_by_return",
                Summary,
                "of those, the ones nothing it runs before it returns may call",
            ),
            CallingWrittenArguments => (
                "Calling.written_arguments",
                Summary,
                "the `fmt::Arguments` parameters it writes into a writer a piece at a time, itself or by what it gives them to: a call gives each made there as the function that writes into the sink it claims (#1652)",
            ),
            LibraryWithThisArguments => (
                "Library.with_this_arguments",
                Summary,
                "the parameters of each `with_this` function it makes that reads `arguments`",
            ),
            LibraryGivesArgument => (
                "Library.gives_argument",
                Summary,
                "its functions each call of which is what it's given, an untagged enum's `From` that's the variant of its argument among them (ADR 0214, amended)",
            ),
            LibraryDisplaysValue => (
                "Library.displays_value",
                Summary,
                "its untagged enums whose `Display` writes each variant's value as it is",
            ),
            LibraryPartialCmpIsCmp => (
                "Library.partial_cmp_is_cmp",
                Summary,
                "its `PartialOrd` impls whose `partial_cmp` is `Some(self.cmp(other))`",
            ),
            LibraryIdentities => (
                "Library.identities",
                Summary,
                "its types equal only to themselves, by their impls",
            ),
            LibraryEqByAddress => (
                "Library.eq_by_address",
                Summary,
                "its types whose `==` is their address's",
            ),
            LibraryAssociated => (
                "Library.associated",
                Summary,
                "each of its traits' associated types whose facts its dictionaries carry, `Size` and `Align` among them (ADR 0145, #1420)",
            ),
            ParamFactUnboxed => (
                "ParamFact::Unboxed",
                ClosedWorld(CompileOwn("compiles its own of the function (ADR 0051)")),
                "none of its crate's calls gives it what may look like `None`, so a `Some` of it is the value",
            ),
            ParamFactCounted => (
                "ParamFact::Counted",
                ClosedWorld(CompileOwn(
                    "compiles its own of the function where it counts what it gives otherwise, or where two such pointees hold it (#1234)",
                )),
                "its JS counts an `Rc` of it, an `Rc` or `Weak` its MIR holds whose pointee holds it, as its crate's own calls decide",
            ),
            CallingNulledParams => (
                "Calling.nulled_params",
                ClosedWorld(Agree(
                    "gives `?? null` of what may be `undefined`; its use as a value is refused (ADR 0861)",
                )),
                "each of its crate's calls gives these nullable parameters `null` or a value",
            ),
            LibraryKeepsCapacity => (
                "Library.keeps_capacity",
                ClosedWorld(CompileOwn(
                    "keeps its own; compiles its own of a function of a library that keeps none (#1180)",
                )),
                "whether anything it compiles reads a `Vec`'s capacity, or has a reader of its own, which std's `read_to_end` shows it to (#1297)",
            ),
            LibraryWritersTakeOptions => (
                "Library.writers_take_options",
                ClosedWorld(CompileOwn("compiles its own of each writer that takes fewer (#1026)")),
                "whether any of its placeholders gives a `Formatter`'s options",
            ),
            LibraryDebugHex => (
                "Library.debug_hex",
                ClosedWorld(CompileOwn("compiles its own of each writer (#1026)")),
                "whether an integer's `{:?}` its writers write may be hex",
            ),
            LibraryDynDebugPretty => (
                "Library.dyn_debug_pretty",
                ClosedWorld(Agree(
                    "its writers of a type it makes a `dyn Debug` take that option (#1335)",
                )),
                "whether it shows a `dyn Debug` it's given pretty",
            ),
            LibraryDynDebugHex => (
                "Library.dyn_debug_hex",
                ClosedWorld(Agree("as `dyn_debug_pretty`")),
                "as `dyn_debug_pretty`, in hex",
            ),
            LibraryReadsFormatterOptions => (
                "Library.reads_formatter_options",
                ClosedWorld(CompileOwn(
                    "compiles its own of each, where its writers take more (#1026)",
                )),
                "its functions that read a `Formatter`'s options",
            ),
            LibraryWritesMayFail => (
                "Library.writes_may_fail",
                ClosedWorld(Agree(
                    "its `write_all` of the library's writer, or of what's given a dictionary, may fail (#1072, #1392)",
                )),
                "whether any write of its may fail",
            ),
            LibraryMakesDroppingAnys => (
                "Library.makes_dropping_anys",
                ClosedWorld(Agree("its `dyn Any`s carry a drop (ADR 0691)")),
                "whether it makes a `dyn Any` of a value that drops",
            ),
            LibraryMakesDroppingErrors => (
                "Library.makes_dropping_errors",
                ClosedWorld(Agree("its `dyn Error`s and `io::Error`s carry a drop (ADR 0462)")),
                "whether it makes a `dyn Error` of a value that drops",
            ),
            LibraryMakesDroppingFnDyns => (
                "Library.makes_dropping_fn_dyns",
                ClosedWorld(Agree("a `dyn Fn` it's handed carries a drop (ADR 0626)")),
                "whether it makes a `dyn Fn` holding a value that drops",
            ),
            LibraryMakesDroppingIterators => (
                "Library.makes_dropping_iterators",
                ClosedWorld(Agree("its `Box<dyn Iterator>` carries a drop (ADR 0521)")),
                "whether it boxes a `dyn Iterator` of a value that drops",
            ),
            LibraryDropsAny => (
                "Library.drops_any",
                ClosedWorld(Agree(
                    "gives generic code drops though none of its own values drops, as the library's generic impls' methods it compiles may be given the library's (ADR 0451, amended, #1591)",
                )),
                "whether any of its own values drops something: a destructor, a counted `Rc`'s count or a guard's borrow; a manifest without it reads as true",
            ),
            LibraryPrintingTypes => (
                "Library.printing_types",
                ClosedWorld(Agree(
                    "writes a line that shows one through stdout's line, and its writers that show one through a sink, as of its own types (ADR 1130, amended, #1608)",
                )),
                "its types one of whose writers may print to stdout as it formats (`Prints::types`)",
            ),
            LibraryStoppingTypes => (
                "Library.stopping_types",
                ClosedWorld(Agree(
                    "writes a std value's `Debug` of one through std's builders (ADR 1130, amended, #1608)",
                )),
                "its types a std value's `Debug` shows piece by piece (`Prints::stopping`)",
            ),
            LibraryStoppingWhereCaught => (
                "Library.stopping_where_caught",
                ClosedWorld(Refuse(
                    "one that catches a panic and shows one in stdout's line is refused, naming #1530 (ADR 1130, amended, #1608)",
                )),
                "its types one of whose writers may stop part-way by a panic, were a consumer to catch it (`Prints::stopping_where_caught`)",
            ),
            LibraryWrittenInto => (
                "Library.written_into",
                Summary,
                "its types whose formatting impls' dictionaries carry `fmtInto`, which writes a value into a writer a piece at a time (`WrittenParams::local_adts`): those it writes so, and each it publishes, reachable from outside it or hidden by an `impl Trait` it gives out, generic ones too (`Prints::published`, #1712, #1743); of `@rust-js/std`, each std type whose writer it compiles, `Fuse<I>`'s (#1712)",
            ),
            LibraryWrittenTraitArguments => (
                "Library.written_trait_arguments",
                Summary,
                "of each of its traits' methods an impl of its writes so, the `fmt::Arguments` parameters it writes into a writer a piece at a time (`WrittenArguments::trait_methods`): a consumer's call through a `dyn` or generic code gives the function that writes into the sink it claims, as `Calling.written_arguments` says of an item",
            ),
            LibraryFramedWriters => (
                "Library.framed_writers",
                Summary,
                "its writer functions, given a `Formatter`, not a formatting impl's, that write a piece at a time (`Prints::framed`): a consumer's framed writer arms one, and its own copy of one writes so (`Foreign::frames_writer_fn`, ADR 1135)",
            ),
            LibraryDynWrittenInto => (
                "Library.dyn_written_into",
                Summary,
                "whether it writes a `dyn` of a formatting trait it's given into a writer a piece at a time, so a consumer's values made one carry `fmtInto`",
            ),
            LibraryMakesFmtDyns => (
                "Library.makes_fmt_dyns",
                ClosedWorld(Refuse(
                    "one that writes a `dyn` into a writer a piece at a time is refused, naming #1688 (ADR 1135, step 5)",
                )),
                "whether it makes a `dyn` of a formatting trait whose dictionary may carry no `fmtInto`",
            ),
            LibraryLeavesUnended => (
                "Library.leaves_unended",
                ClosedWorld(Agree(
                    "prints each whole line through what the runtime holds, `$print` (ADR 1130, amended, #1599, #1608)",
                )),
                "whether its prints may leave stdout's line, and stderr's, unended",
            ),
            LibraryDictionaryConsts => (
                "Library.dictionary_consts",
                ClosedWorld(Refuse("one that reads another is refused (#597)")),
                "a program's: the trait constants its own generic code reads, which its dictionaries carry",
            ),
            LibraryDynsDropNothing => (
                "Library.dyns_drop_nothing",
                ClosedWorld(Refuse("one that makes a `dyn` of what drops is refused (#597)")),
                "a program's: its own traits whose `dyn`s drop nothing",
            ),
            LibraryCountedRcs => (
                "Library.counted_rcs",
                ClosedWorld(Agree("counts each as it does (ADR 0852)")),
                "what the `Rc`s it counts point at",
            ),
            LibraryChanged => (
                "Library.changed",
                ClosedWorld(CompileOwn(
                    "compiles its own of a function where it may copy what the consumer changes and it doesn't (ADR 0914)",
                )),
                "what its code changes in place, which its JS copies where Rust copies",
            ),
            LibraryMergesIdentities => (
                "Library.merges_identities",
                ClosedWorld(CompileOwn(
                    "compiles its own of each, where it compares a function's identity (#1052)",
                )),
                "its functions whose JS may give one function where Rust makes two, as it compares none",
            ),
            LibraryBuilt => (
                "Library.built",
                ClosedWorld(Agree("reads those fields in place (ADR 1022)")),
                "the structs its code makes values of, so their fields are plain data",
            ),
            LibraryClonedClosures => (
                "Library.cloned_closures",
                ClosedWorld(Agree("clones one as the library made it (#1202)")),
                "each closure it clones, and whether apart",
            ),
            LibraryRefCells => ("Library.ref_cells", Holds, "`RefCellFacts`"),
            RefCellFactsBoxed => (
                "RefCellFacts.boxed",
                ClosedWorld(Agree("holds each so, or is refused (ADR 0863)")),
                "its `RefCell`s of an object held as `{ value }`, not the object",
            ),
            RefCellFactsCounted => (
                "RefCellFacts.counted",
                ClosedWorld(Agree("checks each borrow of one (ADR 0863)")),
                "those some borrow of which it counts",
            ),
            RefCellFactsUnchecked => (
                "RefCellFacts.unchecked",
                ClosedWorld(Refuse(
                    "holds no borrow of one while the library's code runs, or is refused (ADR 0863)",
                )),
                "those none of whose borrows can meet another",
            ),
            RefCellFactsHanded => (
                "RefCellFacts.handed",
                ClosedWorld(Agree("shares each with it as it is (ADR 0863)")),
                "those its types hold and its exports take and give",
            ),
            LibraryFields => (
                "Library.fields",
                Holds,
                "each struct's field its consumers read a fact of: `FieldFact`",
            ),
            LibraryTraitMethods => (
                "Library.trait_methods",
                Summary,
                "each of its traits' methods that asks facts of its own slots, and what it asks of each, `[slot, \"ConstSize\"]`, in the order its JS of the method takes them (ADR 0145, #1612); a call through the library's exported impl's dictionary gives these, never facts its own analysis of the default's body finds",
            ),
            FieldFactQuiet => (
                "FieldFact::Quiet",
                ClosedWorld(Refuse(
                    "lends none, and holds no borrow of one while code that may borrow it runs; lending one is refused (#1054)",
                )),
                "a `RefCell` field none of whose borrows its code checks",
            ),
            FieldFactRustFunctions => (
                "FieldFact::RustFunctions",
                ClosedWorld(Agree("its call of one leaves out a trailing `undefined` (ADR 0330)")),
                "each fn pointer the field holds is a Rust function",
            ),
        }
    }
}

/// Of a struct of the contract, each of its fields by its `ContractField`:
/// a pattern naming every field, so one the struct gains is rustc's error
/// here until it's listed, and the list, in the struct's order.
macro_rules! fields {
    ($struct:ident: $($field:ident => $fact:ident,)*) => {
        impl $struct {
            fn named(&self) {
                let $struct { $($field),* } = self;
                $(let _ = $field;)*
            }

            const FIELDS: &[ContractField] = &[$($fact),*];
        }
    };
}

/// Of a set of facts of the contract, each of its variants by its
/// `ContractField`: an exhaustive match, so one the set gains is rustc's
/// error here, and the list, in the set's order.
macro_rules! variants {
    ($set:ident: $($variant:ident => $fact:ident,)*) => {
        impl $set {
            fn field(self) -> ContractField {
                match self {
                    $($set::$variant => $fact),*
                }
            }

            const VARIANTS: &[$set] = &[$($set::$variant),*];
            const FIELDS: &[ContractField] = &[$($fact),*];
        }
    };
}

fields! {
    Library:
    version => LibraryVersion,
    name => LibraryName,
    crate_id => LibraryCrateId,
    crate_hash => LibraryCrateHash,
    items => LibraryItems,
    impls => LibraryImpls,
    libraries => LibraryLibraries,
    inputs => LibraryInputs,
    keeps_capacity => LibraryKeepsCapacity,
    writes_may_fail => LibraryWritesMayFail,
    writers_take_options => LibraryWritersTakeOptions,
    debug_hex => LibraryDebugHex,
    dyn_debug_pretty => LibraryDynDebugPretty,
    dyn_debug_hex => LibraryDynDebugHex,
    reads_formatter_options => LibraryReadsFormatterOptions,
    merges_identities => LibraryMergesIdentities,
    makes_dropping_anys => LibraryMakesDroppingAnys,
    makes_dropping_errors => LibraryMakesDroppingErrors,
    makes_dropping_fn_dyns => LibraryMakesDroppingFnDyns,
    makes_dropping_iterators => LibraryMakesDroppingIterators,
    drops_any => LibraryDropsAny,
    printing_types => LibraryPrintingTypes,
    stopping_types => LibraryStoppingTypes,
    stopping_where_caught => LibraryStoppingWhereCaught,
    leaves_unended => LibraryLeavesUnended,
    written_into => LibraryWrittenInto,
    framed_writers => LibraryFramedWriters,
    written_trait_arguments => LibraryWrittenTraitArguments,
    dyn_written_into => LibraryDynWrittenInto,
    makes_fmt_dyns => LibraryMakesFmtDyns,
    dictionary_consts => LibraryDictionaryConsts,
    dyns_drop_nothing => LibraryDynsDropNothing,
    held => LibraryHeld,
    reexported => LibraryReexported,
    modules => LibraryModules,
    counted_rcs => LibraryCountedRcs,
    with_this_arguments => LibraryWithThisArguments,
    changed => LibraryChanged,
    ref_cells => LibraryRefCells,
    identities => LibraryIdentities,
    keyed => LibraryKeyed,
    eq_by_address => LibraryEqByAddress,
    gives_argument => LibraryGivesArgument,
    displays_value => LibraryDisplaysValue,
    partial_cmp_is_cmp => LibraryPartialCmpIsCmp,
    built => LibraryBuilt,
    cloned_closures => LibraryClonedClosures,
    fields => LibraryFields,
    trait_methods => LibraryTraitMethods,
    associated => LibraryAssociated,
    types => LibraryTypes,
    package => LibraryPackage,
}

fields! {
    Calling:
    params => CallingParams,
    facts => CallingFacts,
    owned_by_return => CallingOwnedByReturn,
    quiet_by_return => CallingQuietByReturn,
    nulled_params => CallingNulledParams,
    written_arguments => CallingWrittenArguments,
}

fields! {
    TypeParam:
    index => TypeParamIndex,
    marks => TypeParamMarks,
    facts => TypeParamFacts,
    stepped => TypeParamStepped,
    built => TypeParamBuilt,
}

fields! {
    RefCellFacts:
    boxed => RefCellFactsBoxed,
    counted => RefCellFactsCounted,
    unchecked => RefCellFactsUnchecked,
    handed => RefCellFactsHanded,
}

variants! {
    ItemFact:
    Fails => ItemFactFails,
    TakesLocation => ItemFactTakesLocation,
    Arguments => ItemFactArguments,
    This => ItemFactThis,
    Rest => ItemFactRest,
    Lazy => ItemFactLazy,
    Variable => ItemFactVariable,
}

variants! {
    ParamFact:
    Drop => ParamFactDrop,
    Copy => ParamFactCopy,
    Clone => ParamFactClone,
    NoDrop => ParamFactNoDrop,
    Unboxed => ParamFactUnboxed,
    ShownPretty => ParamFactShownPretty,
    ShownHex => ParamFactShownHex,
    Counted => ParamFactCounted,
    WrittenInto => ParamFactWrittenInto,
}

variants! {
    FieldFact:
    Quiet => FieldFactQuiet,
    RustFunctions => FieldFactRustFunctions,
}

/// Each fact of the contract, in the order of its types' fields and its
/// sets' variants: the table's rows.
pub(crate) fn every_field() -> impl Iterator<Item = ContractField> {
    [
        Library::FIELDS,
        Calling::FIELDS,
        TypeParam::FIELDS,
        RefCellFacts::FIELDS,
        ItemFact::FIELDS,
        ParamFact::FIELDS,
        FieldFact::FIELDS,
    ]
    .into_iter()
    .flatten()
    .copied()
}

/// The table of docs/library-contract.md, a row of each fact.
pub(crate) fn table() -> String {
    let mut table = String::from("| Fact | Scope | What it says | A consumer that differs |\n|---|---|---|---|\n");
    for field in every_field() {
        let (name, scope, says) = field.row();
        let (scope, differs) = match scope {
            FactScope::Local => ("local", ""),
            FactScope::Summary => ("summary", ""),
            FactScope::Holds => ("holds", ""),
            FactScope::ClosedWorld(Differs::Agree(how)) => ("closed-world: agrees", how),
            FactScope::ClosedWorld(Differs::CompileOwn(how)) => ("closed-world: compiles its own", how),
            FactScope::ClosedWorld(Differs::Refuse(how)) => ("closed-world: refused", how),
        };
        let differs = if differs.is_empty() {
            String::from(" ")
        } else {
            format!(" {differs} ")
        };
        table.push_str(&format!("| `{name}` | {scope} | {says} |{differs}|\n"));
    }
    table
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Each fact is listed once, by each struct's fields and each set's
    /// variants, each with what it says, a closed-world one with what a
    /// consumer that differs does.
    #[test]
    fn each_fact_says_what_makes_it_true() {
        let manifest: serde_json::Value = serde_json::from_str(super::super::STD_MANIFEST).expect("std's manifest");
        let library: Library = serde_json::from_value(manifest["library"].clone()).expect("a contract");
        library.named();
        Calling::default().named();
        TypeParam::default().named();
        RefCellFacts::default().named();
        assert_eq!(
            ItemFact::VARIANTS.iter().map(|v| v.field()).collect::<Vec<_>>(),
            ItemFact::FIELDS
        );
        assert_eq!(
            ParamFact::VARIANTS.iter().map(|v| v.field()).collect::<Vec<_>>(),
            ParamFact::FIELDS
        );
        assert_eq!(
            FieldFact::VARIANTS.iter().map(|v| v.field()).collect::<Vec<_>>(),
            FieldFact::FIELDS
        );
        let fields: Vec<ContractField> = every_field().collect();
        for (at, field) in fields.iter().enumerate() {
            assert!(!fields[..at].contains(field), "{field:?} listed twice");
            let (name, scope, says) = field.row();
            assert!(!says.is_empty(), "{name} says nothing");
            if let FactScope::ClosedWorld(Differs::Agree(how) | Differs::CompileOwn(how) | Differs::Refuse(how)) = scope
            {
                assert!(!how.is_empty(), "{name} says nothing of a consumer that differs");
            }
        }
        let closed = fields
            .iter()
            .filter(|field| matches!(field.row().1, FactScope::ClosedWorld(_)));
        assert!(closed.count() > 15);
    }

    /// docs/library-contract.md's table is the one `table` makes:
    /// `BLESS=1 cargo test contract_table` writes it.
    #[test]
    fn contract_table() {
        let page = include_str!("../../docs/library-contract.md");
        let (start, end) = (
            "<!-- the facts: src/library/scopes.rs' table -->\n",
            "<!-- end of the facts -->\n",
        );
        let from = page.find(start).expect("the table's start") + start.len();
        let to = page.find(end).expect("the table's end");
        let made = table();
        if std::env::var_os("BLESS").is_some() {
            let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("docs/library-contract.md");
            std::fs::write(path, format!("{}{made}{}", &page[..from], &page[to..])).expect("the page written");
        } else {
            assert_eq!(
                page[from..to],
                made,
                "docs/library-contract.md's table: BLESS=1 cargo test contract_table"
            );
        }
    }
}
