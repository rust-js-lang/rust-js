//! Which std function each of the runtime's models stands for, and what
//! proves it equal to std's own body (Campaign 5, #1458, #2513; ADR 1202):
//! the one catalog of them. A model is a helper of the runtime's, or a form
//! the lowering writes, in place of a std body; glue is rust-js's own, how JS
//! holds a value, a reference, a panic, a drop or a trait's dictionary, no
//! copy of a std body. A model is `Pitted` where the twin net (ADR 1122)
//! runs a case of it against std's compiled body in the build of std's
//! iterator structs, else `Unproven` for its reason (`Why`). Its std items,
//! `stands_for`, are paths through std's public modules, each resolved where
//! `bun run std` reads std (`unresolved`), so one that names nothing fails
//! there; none yet of one not bound, which a later step binds. The compiler
//! prints it, `rust-js --std-models`, which the twin net reads: no second
//! list (test/architecture-models.test.ts).

pub(crate) mod rows;

use rustc_hir::def::DefKind;
use rustc_middle::ty::TyCtxt;
use rustc_span::def_id::DefId;

/// Why no case pits a model against std's body: std's build of its
/// iterator structs has no body of its own in its place.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Why {
    /// a std function no table of the twin net reaches yet: a `Duration`'s, UTF-8's, an I/O type's, or an iterator's no table has a case of
    Outside,
    /// an `Iterator` method std provides, whose default body std's build lowers as the plain build does: it compiles std's adapter structs, not the consumers
    Consumers,
    /// a std iterator whose struct std's build doesn't compile, a slice's search
    Iterators,
    /// a std iterator stepped as JS's own iteration by decision, an array's, a string's `Chars`, any JS iterator's step: its ADR beside it (#1458)
    JsOwn,
    /// a std method whose body reaches memory, which std's build can't compile (docs/std-coverage.txt): its ADR beside it
    Memory,
    /// a range's `Iterator`, `Step`'s, which std's build doesn't compile: its literal panics, its specialized traits, its bound's comparisons, each integer's `Step` and a range's every form would need it (#1589)
    Ranges,
    /// a method of `Vec`, `VecDeque`, a slice, `String`, `BinaryHeap` or a map or a set, whose body std's build doesn't compile: it compiles std's iterator structs alone
    Collections,
    /// a sort, which rust-js writes as its own in both builds
    Sort,
    /// `Debug`, which std's build doesn't compile
    Format,
    /// a `RefCell`'s borrows and its guards, a map of one too, which std's build doesn't compile
    Cells,
    /// a number parsed from text, its `FromStr`, which std's build doesn't compile: the tables' `String` items' `v()` (#1681's step (a))
    Numbers,
}

impl Why {
    /// Its name, as the twin net says it.
    pub fn name(self) -> &'static str {
        match self {
            Why::Outside => "outside",
            Why::Consumers => "consumers",
            Why::Iterators => "iterators",
            Why::JsOwn => "js-own",
            Why::Memory => "memory",
            Why::Ranges => "ranges",
            Why::Collections => "collections",
            Why::Sort => "sort",
            Why::Format => "format",
            Why::Cells => "cells",
            Why::Numbers => "numbers",
        }
    }

    /// What it says.
    pub fn says(self) -> &'static str {
        match self {
            Why::Outside => {
                "a std function no table of the twin net reaches yet: a `Duration`'s, UTF-8's, an I/O type's, or an iterator's no table has a case of"
            }
            Why::Consumers => {
                "an `Iterator` method std provides, whose default body std's build lowers as the plain build does: it compiles std's adapter structs, not the consumers"
            }
            Why::Iterators => "a std iterator whose struct std's build doesn't compile, a slice's search",
            Why::JsOwn => {
                "a std iterator stepped as JS's own iteration by decision, an array's, a string's `Chars`, any JS iterator's step: its ADR beside it (#1458)"
            }
            Why::Memory => {
                "a std method whose body reaches memory, which std's build can't compile (docs/std-coverage.txt): its ADR beside it"
            }
            Why::Ranges => {
                "a range's `Iterator`, `Step`'s, which std's build doesn't compile: its literal panics, its specialized traits, its bound's comparisons, each integer's `Step` and a range's every form would need it (#1589)"
            }
            Why::Collections => {
                "a method of `Vec`, `VecDeque`, a slice, `String`, `BinaryHeap` or a map or a set, whose body std's build doesn't compile: it compiles std's iterator structs alone"
            }
            Why::Sort => "a sort, which rust-js writes as its own in both builds",
            Why::Format => "`Debug`, which std's build doesn't compile",
            Why::Cells => "a `RefCell`'s borrows and its guards, a map of one too, which std's build doesn't compile",
            Why::Numbers => {
                "a number parsed from text, its `FromStr`, which std's build doesn't compile: the tables' `String` items' `v()` (#1681's step (a))"
            }
        }
    }
}

/// What glue is, as the twin net says it.
pub const GLUE: &str =
    "rust-js's own: how JS holds a value, a reference, a panic, a drop or a trait's dictionary, no copy of a std body";

/// What proves a model equal to std's body.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Proof {
    /// The twin net runs a case of it against std's compiled body.
    Pitted,
    /// Nothing yet, for this reason.
    Unproven(Why),
}

/// A std item a model stands for: at its path through std's public
/// modules, or the function whose panics a row of `$stdSites` holds, the
/// row by its name (`rows::ROWS`, or one the std build writes beside it).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Stands {
    Path(&'static str),
    Row(&'static str),
}

/// What a helper is: glue, or a model of the std items it stands for,
/// none yet where it isn't bound; or a form the lowering writes in place
/// of a std body, which reads its rows of `$stdSites`, by the row's name.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Kind {
    Glue,
    Model(Proof, &'static [Stands]),
    Form(&'static [Stands]),
}

/// The net that holds a row of the catalog: whose run says it's right, and
/// so whose drift may take it off. The twin net's whole run takes a row it
/// holds off where no case imports it; a row another net holds it leaves
/// alone, as its own net answers for it (ADR 1202, amended).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Net {
    /// The twin net (ADR 1122): a case of it pits the helper against std's
    /// body, or imports it in both builds.
    Twin,
    /// The boundary's tests (ADR 1192): a check of a value JS gives, which
    /// no twin case reaches, as a case's functions take no value from JS.
    Boundary,
    /// `bun run std`'s sites (test/std-sites.test.ts): a model that reads
    /// rows of `$stdSites`, std's sites of its function's panics, which no
    /// table of the twin net reaches yet.
    StdSites,
}

impl Net {
    /// Its name, as the twin net reads it.
    pub fn name(self) -> &'static str {
        match self {
            Net::Twin => "twin",
            Net::Boundary => "boundary",
            Net::StdSites => "std-sites",
        }
    }
}

/// One of the runtime's helpers, by its name, the export `$` and it, what
/// it is, a note, its ADR, and the net that holds it.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Entry {
    pub helper: &'static str,
    pub kind: Kind,
    pub note: &'static str,
    pub held: Net,
}

impl Entry {
    /// The row, held by `net`, not the twin net.
    pub const fn held(self, net: Net) -> Entry {
        Entry { held: net, ..self }
    }
}

use Kind::{Form, Glue, Model};
use Proof::{Pitted, Unproven};
use Stands::{Path, Row};

/// A row of the catalog, which the twin net holds; `.held(net)` says
/// another net does.
const fn entry(helper: &'static str, kind: Kind, note: &'static str) -> Entry {
    Entry {
        helper,
        kind,
        note,
        held: Net::Twin,
    }
}

/// The catalog, by helper, each on one line, which the twin net's bless
/// writes (`test/twin-net.ts`' `blessedCatalog`).
#[rustfmt::skip]
pub const MODELS: &[Entry] = &[
    entry("BTreeMap", Model(Unproven(Why::Collections), &[]), ""),
    entry("BTreeSet", Model(Unproven(Why::Collections), &[]), ""),
    entry("CheckFailure", Glue, "ADR 1192: a check of a value JS gives a Rust parameter, at the boundary").held(Net::Boundary),
    entry("KeyMap", Model(Unproven(Why::Collections), &[]), ""),
    entry("KeySet", Model(Unproven(Why::Collections), &[]), ""),
    entry("abort", Model(Unproven(Why::Outside), &[Row("panicNounwindNobacktrace")]), "its rows of `$stdSites`, std's sites of its function's panics").held(Net::StdSites),
    entry("add", Model(Unproven(Why::Collections), &[]), ""),
    entry("advanceSlice", Model(Unproven(Why::JsOwn), &[]), "ADRs 0422, 0465: an array's iterator an adapter holds, advanced by index"),
    entry("andModify", Model(Unproven(Why::Collections), &[]), ""),
    entry("append", Model(Unproven(Why::Collections), &[]), ""),
    entry("asChunksMut", Model(Unproven(Why::Collections), &[]), ""),
    entry("asciiCase", Model(Unproven(Why::Collections), &[]), ""),
    entry("assign", Glue, ""),
    entry("at", Glue, ""),
    entry("bidirectionalMerge", Model(Unproven(Why::Sort), &[Row("sortTotalOrder")]), "its rows of `$stdSites`, std's sites of its function's panics"),
    entry("binarySearchBy", Model(Unproven(Why::Collections), &[]), ""),
    entry("borrow", Model(Unproven(Why::Cells), &[Path("core::cell::RefCell::borrow")]), ""),
    entry("borrowMut", Model(Unproven(Why::Cells), &[Path("core::cell::RefCell::borrow_mut")]), ""),
    entry("brand", Glue, "ADR 1196: a brand of a type no other crate can make").held(Net::Boundary),
    entry("brandOf", Glue, "ADR 1196: a brand of a type no other crate can make").held(Net::Boundary),
    entry("brandsKey", Glue, "ADR 1196: a brand of a type no other crate can make").held(Net::Boundary),
    entry("btreeSet", Model(Unproven(Why::Collections), &[]), ""),
    entry("bufReader", Model(Unproven(Why::Outside), &[Row("bufReaderStreamPosition")]), "its rows of `$stdSites`, std's sites of its function's panics").held(Net::StdSites),
    entry("byteLen", Model(Unproven(Why::Collections), &[]), ""),
    entry("cannotUnwind", Model(Unproven(Why::Outside), &[Row("panicNounwind")]), "its rows of `$stdSites`, std's sites of its function's panics").held(Net::StdSites),
    entry("capacity", Model(Unproven(Why::Collections), &[]), ""),
    entry("catchUnwind", Glue, ""),
    entry("catchable", Glue, ""),
    entry("chars", Model(Unproven(Why::JsOwn), &[Path("core::str::Chars")]), "ADR 1117: a string's `Chars`, the runtime's lazy one"),
    entry("checkAccessor", Glue, "ADR 1194: a check of a value JS gives that holds others, whole").held(Net::Boundary),
    entry("checkArray", Glue, "ADR 1194: a check of a value JS gives that holds others, whole").held(Net::Boundary),
    entry("checkAt", Glue, "ADR 1194: a check of a value JS gives that holds others, whole").held(Net::Boundary),
    entry("checkBig", Glue, "ADR 1192: a check of a value JS gives a Rust parameter, at the boundary").held(Net::Boundary),
    entry("checkBool", Glue, "ADR 1192: a check of a value JS gives a Rust parameter, at the boundary").held(Net::Boundary),
    entry("checkBoxedOption", Glue, "ADR 1192: a check of a value JS gives a Rust parameter, at the boundary").held(Net::Boundary),
    entry("checkBrand", Glue, "ADR 1196: a brand of a type no other crate can make").held(Net::Boundary),
    entry("checkCallback", Glue, "ADR 1195: a check of what a JS function gives back").held(Net::Boundary),
    entry("checkCell", Glue, "ADR 1194: a check of a value JS gives that holds others, whole").held(Net::Boundary),
    entry("checkChar", Glue, "ADR 1192: a check of a value JS gives a Rust parameter, at the boundary").held(Net::Boundary),
    entry("checkF32", Glue, "ADR 1192: a check of a value JS gives a Rust parameter, at the boundary").held(Net::Boundary),
    entry("checkF64", Glue, "ADR 1192: a check of a value JS gives a Rust parameter, at the boundary").held(Net::Boundary),
    entry("checkFailed", Glue, "ADR 1192: a check of a value JS gives a Rust parameter, at the boundary").held(Net::Boundary),
    entry("checkField", Glue, "ADR 1194: a check of a value JS gives that holds others, whole").held(Net::Boundary),
    entry("checkI128", Glue, "ADR 1192: a check of a value JS gives a Rust parameter, at the boundary").held(Net::Boundary),
    entry("checkI16", Glue, "ADR 1192: a check of a value JS gives a Rust parameter, at the boundary").held(Net::Boundary),
    entry("checkI32", Glue, "ADR 1192: a check of a value JS gives a Rust parameter, at the boundary").held(Net::Boundary),
    entry("checkI64", Glue, "ADR 1192: a check of a value JS gives a Rust parameter, at the boundary").held(Net::Boundary),
    entry("checkI8", Glue, "ADR 1192: a check of a value JS gives a Rust parameter, at the boundary").held(Net::Boundary),
    entry("checkInt", Glue, "ADR 1192: a check of a value JS gives a Rust parameter, at the boundary").held(Net::Boundary),
    entry("checkIntRanges", Glue, "ADR 1194: a check of a value JS gives that holds others, whole").held(Net::Boundary),
    entry("checkItem", Glue, "ADR 1194: a check of a value JS gives that holds others, whole").held(Net::Boundary),
    entry("checkLeaves", Glue, "ADR 1192: a check of a value JS gives a Rust parameter, at the boundary").held(Net::Boundary),
    entry("checkMap", Glue, "ADR 1194: a check of a value JS gives that holds others, whole").held(Net::Boundary),
    entry("checkNone", Glue, "ADR 1192: a check of a value JS gives a Rust parameter, at the boundary").held(Net::Boundary),
    entry("checkObject", Glue, "ADR 1194: a check of a value JS gives that holds others, whole").held(Net::Boundary),
    entry("checkOneOf", Glue, "ADR 1192: a check of a value JS gives a Rust parameter, at the boundary").held(Net::Boundary),
    entry("checkOption", Glue, "ADR 1192: a check of a value JS gives a Rust parameter, at the boundary").held(Net::Boundary),
    entry("checkPlace", Glue, "ADR 1194: a check of a value JS gives that holds others, whole").held(Net::Boundary),
    entry("checkRead", Glue, "ADR 1194: a check of a value JS gives that holds others, whole").held(Net::Boundary),
    entry("checkResult", Glue, "ADR 1195: a check of what a JS function gives back").held(Net::Boundary),
    entry("checkSet", Glue, "ADR 1194: a check of a value JS gives that holds others, whole").held(Net::Boundary),
    entry("checkShape", Glue, "ADR 1194: a check of a value JS gives that holds others, whole").held(Net::Boundary),
    entry("checkShown", Glue, "ADR 1192: a check of a value JS gives a Rust parameter, at the boundary").held(Net::Boundary),
    entry("checkStr", Glue, "ADR 1192: a check of a value JS gives a Rust parameter, at the boundary").held(Net::Boundary),
    entry("checkTag", Glue, "ADR 1194: a check of a value JS gives that holds others, whole").held(Net::Boundary),
    entry("checkTuple", Glue, "ADR 1194: a check of a value JS gives that holds others, whole").held(Net::Boundary),
    entry("checkU128", Glue, "ADR 1192: a check of a value JS gives a Rust parameter, at the boundary").held(Net::Boundary),
    entry("checkU16", Glue, "ADR 1192: a check of a value JS gives a Rust parameter, at the boundary").held(Net::Boundary),
    entry("checkU32", Glue, "ADR 1192: a check of a value JS gives a Rust parameter, at the boundary").held(Net::Boundary),
    entry("checkU64", Glue, "ADR 1192: a check of a value JS gives a Rust parameter, at the boundary").held(Net::Boundary),
    entry("checkU8", Glue, "ADR 1192: a check of a value JS gives a Rust parameter, at the boundary").held(Net::Boundary),
    entry("checkUnit", Glue, "ADR 1192: a check of a value JS gives a Rust parameter, at the boundary").held(Net::Boundary),
    entry("checkWhole", Glue, "ADR 1194: a check of a value JS gives that holds others, whole").held(Net::Boundary),
    entry("checkedBefore", Glue, "ADR 1194: a check of a value JS gives that holds others, whole").held(Net::Boundary),
    entry("checkedSplitAtMut", Model(Unproven(Why::Collections), &[]), ""),
    entry("checking", Glue, "ADR 1194: a check of a value JS gives that holds others, whole").held(Net::Boundary),
    entry("checks", Glue, "ADR 1192: a check of a value JS gives a Rust parameter, at the boundary").held(Net::Boundary),
    entry("chunksExactMut", Model(Unproven(Why::Collections), &[]), ""),
    entry("chunksMut", Model(Unproven(Why::Collections), &[]), ""),
    entry("clampBy", Model(Unproven(Why::Collections), &[]), ""),
    entry("clampFloat", Model(Unproven(Why::Outside), &[Row("clamp")]), "its rows of `$stdSites`, std's sites of its function's panics").held(Net::StdSites),
    entry("cleanup", Glue, ""),
    entry("clone", Model(Pitted, &[]), ""),
    entry("cloneFromSlice", Model(Unproven(Why::Collections), &[]), ""),
    entry("cloneIntoVec", Model(Unproven(Why::Collections), &[]), ""),
    entry("clonedAt", Form(&[Row("clonedAt")]), "the lowering's, std_impls/clones.rs"),
    entry("closing", Glue, ""),
    entry("cmp", Glue, ""),
    entry("cmpItems", Model(Unproven(Why::Consumers), &[]), ""),
    entry("collectDropping", Model(Pitted, &[Path("core::iter::Iterator::collect")]), ""),
    entry("collectOptions", Model(Unproven(Why::Consumers), &[]), ""),
    entry("collectResults", Model(Unproven(Why::Consumers), &[]), ""),
    entry("collectVec", Model(Unproven(Why::Consumers), &[]), ""),
    entry("collectsInPlace", Model(Unproven(Why::Consumers), &[]), ""),
    entry("consume", Model(Pitted, &[]), ""),
    entry("copied", Model(Unproven(Why::Outside), &[Row("copiedCount"), Row("copiedLast")]), "its rows of `$stdSites`, std's sites of its function's panics").held(Net::StdSites),
    entry("copyFromSlice", Model(Unproven(Why::Collections), &[]), ""),
    entry("copyRange", Model(Unproven(Why::Collections), &[]), ""),
    entry("copyWithin", Model(Unproven(Why::Collections), &[]), ""),
    entry("count", Model(Unproven(Why::Consumers), &[]), ""),
    entry("cycleOf", Model(Pitted, &[Path("core::iter::Cycle")]), ""),
    entry("debugDuration", Model(Unproven(Why::Format), &[]), ""),
    entry("debugF64", Model(Unproven(Why::Format), &[]), ""),
    entry("debugList", Model(Unproven(Why::Format), &[]), ""),
    entry("debugMap", Model(Unproven(Why::Format), &[]), ""),
    entry("debugParseError", Model(Unproven(Why::Format), &[]), ""),
    entry("debugSet", Model(Unproven(Why::Format), &[]), ""),
    entry("debugStr", Model(Unproven(Why::Format), &[]), ""),
    entry("decodeUtf16", Model(Unproven(Why::Iterators), &[Path("core::char::DecodeUtf16")]), "ADR 0327, #1584: `char::decode_utf16`'s, whose error is its code, where std's body makes `DecodeUtf16Error { code }`"),
    entry("dedupBy", Model(Unproven(Why::Collections), &[]), ""),
    entry("dedupByCells", Model(Unproven(Why::Collections), &[]), ""),
    entry("dequeInsert", Model(Unproven(Why::Collections), &[Row("vecDequeInsert")]), ""),
    entry("dequeSwapRemove", Model(Unproven(Why::Collections), &[]), ""),
    entry("dispose", Glue, ""),
    entry("disposeIterator", Model(Unproven(Why::JsOwn), &[]), "ADR 0140, ADR 1136: a `dyn Iterator`'s JS iterator, a binding's, closed as JS's own"),
    entry("drain", Model(Unproven(Why::Collections), &[]), ""),
    entry("drainAll", Model(Unproven(Why::Collections), &[]), ""),
    entry("dropEach", Glue, ""),
    entry("dropItems", Glue, ""),
    entry("dropping", Glue, ""),
    entry("droppingIterator", Model(Unproven(Why::JsOwn), &[]), "ADR 0140, ADR 1136: a `dyn Iterator`'s JS iterator stepped dropping what it gives, as JS's own"),
    entry("durationAdd", Model(Unproven(Why::Outside), &[Row("durationAdd")]), "its rows of `$stdSites`, std's sites of its function's panics").held(Net::StdSites),
    entry("durationNew", Model(Unproven(Why::Outside), &[Row("durationNew")]), "its rows of `$stdSites`, std's sites of its function's panics").held(Net::StdSites),
    entry("durationSub", Model(Unproven(Why::Outside), &[Row("durationSub")]), "its rows of `$stdSites`, std's sites of its function's panics").held(Net::StdSites),
    entry("encodeCharUtf16", Model(Unproven(Why::Outside), &[Row("encodeUtf16")]), "its rows of `$stdSites`, std's sites of its function's panics").held(Net::StdSites),
    entry("encodeUtf8", Model(Unproven(Why::Outside), &[Row("encodeUtf8")]), "its rows of `$stdSites`, std's sites of its function's panics").held(Net::StdSites),
    entry("endEntry", Model(Unproven(Why::Collections), &[]), ""),
    entry("entry", Model(Unproven(Why::Collections), &[]), ""),
    entry("entryGet", Model(Unproven(Why::Collections), &[]), ""),
    entry("entryInsert", Model(Unproven(Why::Collections), &[]), ""),
    entry("entryOrInsert", Model(Unproven(Why::Collections), &[]), ""),
    entry("entryOrInsertWith", Model(Unproven(Why::Collections), &[]), ""),
    entry("entryRemove", Model(Unproven(Why::Collections), &[]), ""),
    entry("enumerate", Model(Pitted, &[Path("core::iter::Enumerate"), Row("enumerateCount")]), ""),
    entry("eq", Glue, ""),
    entry("exactLen", Model(Pitted, &[Path("core::iter::ExactSizeIterator::len"), Row("exactSizeLen")]), ""),
    entry("exchange", Glue, ""),
    entry("extend", Model(Unproven(Why::Collections), &[Row("vecExtendTrusted")]), ""),
    entry("extendMap", Model(Unproven(Why::Collections), &[]), ""),
    entry("extendText", Model(Unproven(Why::Collections), &[]), ""),
    entry("extractIf", Model(Unproven(Why::Collections), &[]), ""),
    entry("fill", Model(Unproven(Why::Collections), &[]), ""),
    entry("fillWith", Model(Unproven(Why::Collections), &[]), ""),
    entry("filterDropping", Model(Pitted, &[Path("core::iter::Filter")]), ""),
    entry("filterEnds", Model(Pitted, &[Path("core::iter::Filter")]), ""),
    entry("findBack", Model(Pitted, &[Path("core::iter::DoubleEndedIterator::rfind")]), ""),
    entry("findDropping", Model(Unproven(Why::Consumers), &[]), ""),
    entry("flatLast", Model(Unproven(Why::Outside), &[Row("flattenLast")]), "its rows of `$stdSites`, std's sites of its function's panics").held(Net::StdSites),
    entry("flatMap", Model(Pitted, &[Path("core::iter::FlatMap"), Row("flattenCount")]), ""),
    entry("flattenEnds", Model(Pitted, &[Path("core::iter::Flatten")]), ""),
    entry("fmtPrintFailed", Model(Unproven(Why::Outside), &[Row("printFailed")]), "its rows of `$stdSites`, std's sites of its function's panics").held(Net::StdSites),
    entry("foldBack", Model(Pitted, &[Path("core::iter::DoubleEndedIterator::rfold")]), ""),
    entry("foldDropping", Model(Unproven(Why::Consumers), &[]), ""),
    entry("foldOf", Model(Unproven(Why::Consumers), &[]), ""),
    entry("foldToArray", Model(Unproven(Why::Consumers), &[]), ""),
    entry("folded", Model(Unproven(Why::Consumers), &[]), ""),
    entry("force", Model(Unproven(Why::Outside), &[Row("lazyCellPoisoned")]), "its rows of `$stdSites`, std's sites of its function's panics").held(Net::StdSites),
    entry("formatting", Model(Unproven(Why::Format), &[]), ""),
    entry("fromDigit", Model(Unproven(Why::Outside), &[Row("fromDigit")]), "its rows of `$stdSites`, std's sites of its function's panics").held(Net::StdSites),
    entry("getDisjointMut", Model(Unproven(Why::Collections), &[]), ""),
    entry("getOrInit", Model(Unproven(Why::Outside), &[Row("onceCellReentrant")]), "its rows of `$stdSites`, std's sites of its function's panics").held(Net::StdSites),
    entry("given", Glue, ""),
    entry("givenDrop", Glue, ""),
    entry("grew", Model(Unproven(Why::Collections), &[]), ""),
    entry("grownBy", Model(Unproven(Why::Collections), &[Row("capacityOverflow"), Row("vecDequeExtendTrusted"), Row("vecDequeReserve"), Row("vecExtendTrusted")]), ""),
    entry("grownVec", Model(Unproven(Why::Collections), &[]), ""),
    entry("guardFilterMap", Model(Unproven(Why::Cells), &[]), ""),
    entry("guardMap", Model(Unproven(Why::Cells), &[]), ""),
    entry("guardMapSplit", Model(Unproven(Why::Cells), &[]), ""),
    entry("hashSlice", Glue, ""),
    entry("hashedMap", Model(Unproven(Why::Collections), &[]), ""),
    entry("hashedSet", Model(Unproven(Why::Collections), &[]), ""),
    entry("heapAppend", Model(Unproven(Why::Collections), &[]), ""),
    entry("heapExtend", Model(Unproven(Why::Collections), &[]), ""),
    entry("heapFrom", Model(Unproven(Why::Collections), &[]), ""),
    entry("heapPop", Model(Unproven(Why::Collections), &[]), ""),
    entry("heapPush", Model(Unproven(Why::Collections), &[]), ""),
    entry("heapRetain", Model(Unproven(Why::Collections), &[]), ""),
    entry("heapSorted", Model(Unproven(Why::Collections), &[]), ""),
    entry("held", Glue, ""),
    entry("holding", Model(Pitted, &[]), ""),
    entry("inPlaceCapacityFrom", Model(Unproven(Why::Collections), &[]), "#1428: a `Vec` collected in place, its capacity its source buffer's, walked from the iterator std's struct holds"),
    entry("index", Glue, ""),
    entry("insert", Model(Unproven(Why::Collections), &[]), ""),
    entry("insertAt", Model(Unproven(Why::Collections), &[]), ""),
    entry("insertMut", Model(Unproven(Why::Collections), &[]), ""),
    entry("insertStr", Model(Unproven(Why::Collections), &[]), ""),
    entry("intoIter", Model(Unproven(Why::JsOwn), &[]), "ADRs 0061, 0475: a generic `IntoIterator` of an array, its `$iter`"),
    entry("intsOrdered", Glue, ""),
    entry("isSortedBy", Model(Unproven(Why::Consumers), &[]), ""),
    entry("isSortedByKey", Model(Unproven(Why::Consumers), &[]), ""),
    entry("iter", Model(Unproven(Why::JsOwn), &[Path("core::slice::Iter")]), "ADR 0071: an array's iterator, which knows where it is"),
    entry("iterCmp", Model(Unproven(Why::Consumers), &[]), ""),
    entry("iterEq", Model(Unproven(Why::Consumers), &[]), ""),
    entry("iterEqGiven", Model(Unproven(Why::Consumers), &[]), ""),
    entry("iterator", Model(Pitted, &[]), ""),
    entry("jsIterator", Model(Unproven(Why::JsOwn), &[]), "ADR 1136, amended: the dictionary of any JS iterator generic code is given, its steps `$next`'s"),
    entry("jsIteratorSome", Model(Unproven(Why::JsOwn), &[]), "ADRs 1136, 0051: the same of items that may look like `None`, boxed"),
    entry("keepCapacity", Model(Unproven(Why::Collections), &[]), ""),
    entry("knowing", Model(Pitted, &[]), ""),
    entry("last", Model(Unproven(Why::Consumers), &[]), ""),
    entry("lastDropping", Model(Unproven(Why::Consumers), &[]), ""),
    entry("lazyChain", Model(Pitted, &[Path("core::iter::Chain"), Row("chainCount"), Row("fnOnceCall")]), ""),
    entry("lazyChunkBy", Model(Unproven(Why::Iterators), &[Path("core::slice::ChunkBy")]), ""),
    entry("lazyChunkByMut", Model(Unproven(Why::Iterators), &[Path("core::slice::ChunkByMut")]), ""),
    entry("lazyLockPoisoned", Model(Unproven(Why::Outside), &[Row("lazyLockPoisoned")]), "its rows of `$stdSites`, std's sites of its function's panics").held(Net::StdSites),
    entry("lazyMapWhile", Model(Pitted, &[Path("core::iter::MapWhile")]), ""),
    entry("lazyRange", Model(Pitted, &[Path("core::ops::Range")]), ""),
    entry("lazyScan", Model(Pitted, &[Path("core::iter::Scan")]), ""),
    entry("lazySkipWhile", Model(Pitted, &[Path("core::iter::SkipWhile")]), ""),
    entry("lazySliceSplitBy", Model(Unproven(Why::Iterators), &[]), ""),
    entry("lazySliceSplitByMut", Model(Unproven(Why::Iterators), &[]), ""),
    entry("lazyTakeWhile", Model(Pitted, &[Path("core::iter::TakeWhile")]), ""),
    entry("lazyZip", Model(Pitted, &[Path("core::iter::Zip")]), ""),
    entry("lent", Model(Pitted, &[]), ""),
    entry("live", Glue, ""),
    entry("madeEach", Glue, ""),
    entry("madeInTurn", Glue, ""),
    entry("main", Glue, "ADR 1198: a binary's start, which every case's binary imports"),
    entry("makeMut", Glue, ""),
    entry("mapEnds", Model(Pitted, &[Path("core::iter::Map")]), ""),
    entry("mapExtractIf", Model(Unproven(Why::Collections), &[]), ""),
    entry("mapFrom", Model(Unproven(Why::Collections), &[]), ""),
    entry("mapGetDisjointMut", Model(Unproven(Why::Collections), &[Row("mapGetDisjointMut")]), "its row read by the lowering's std_types/map.rs"),
    entry("max", Model(Unproven(Why::Consumers), &[]), ""),
    entry("maxBy", Model(Unproven(Why::Consumers), &[]), ""),
    entry("maxByKey", Model(Unproven(Why::Consumers), &[]), ""),
    entry("maxOf", Model(Unproven(Why::Consumers), &[]), ""),
    entry("min", Model(Unproven(Why::Consumers), &[]), ""),
    entry("minBy", Model(Unproven(Why::Consumers), &[]), ""),
    entry("minByKey", Model(Unproven(Why::Consumers), &[]), ""),
    entry("minOf", Model(Unproven(Why::Consumers), &[]), ""),
    entry("mutAt", Glue, ""),
    entry("mutEntries", Model(Unproven(Why::Collections), &[]), ""),
    entry("mutGet", Model(Unproven(Why::Collections), &[]), ""),
    entry("mutValues", Model(Unproven(Why::Collections), &[]), ""),
    entry("newCyclic", Glue, ""),
    entry("next", Model(Unproven(Why::JsOwn), &[]), "ADR 0140: the step of any JS iterator, its item or `undefined`"),
    entry("nextBack", Model(Pitted, &[Path("core::iter::DoubleEndedIterator::next_back")]), ""),
    entry("nextBackSome", Model(Unproven(Why::JsOwn), &[]), "ADRs 0164, 0051: the step from its back of any JS iterator, its item boxed where it may look like `None`, of an `Option`'s items (#1826)"),
    entry("nextMultipleOf", Model(Unproven(Why::Outside), &[Row("nextMultipleOf")]), "its rows of `$stdSites`, std's sites of its function's panics").held(Net::StdSites),
    entry("nextSome", Model(Unproven(Why::JsOwn), &[]), "ADRs 0140, 0051: the step of any JS iterator, its item boxed where it may look like `None`"),
    entry("nth", Model(Unproven(Why::Consumers), &[]), ""),
    entry("nthBack", Model(Unproven(Why::Consumers), &[]), ""),
    entry("nthDropping", Model(Unproven(Why::Consumers), &[]), ""),
    entry("orInsert", Model(Unproven(Why::Collections), &[]), ""),
    entry("orInsertWith", Model(Unproven(Why::Collections), &[]), ""),
    entry("orInsertWithKey", Model(Unproven(Why::Collections), &[]), ""),
    entry("ord", Glue, ""),
    entry("ordClamp", Form(&[Row("ordClamp")]), "the lowering's, ordering.rs"),
    entry("ordTreeMap", Model(Unproven(Why::Collections), &[]), ""),
    entry("ordTreeSet", Model(Unproven(Why::Collections), &[]), ""),
    entry("out", Model(Unproven(Why::Format), &[]), ""),
    entry("panic", Glue, ""),
    entry("parseBig", Model(Unproven(Why::Numbers), &[Row("fromStrRadix")]), "its rows of `$stdSites`, std's sites of its function's panics"),
    entry("parseInt", Model(Unproven(Why::Numbers), &[Row("fromStrRadix")]), ""),
    entry("partialEq", Glue, ""),
    entry("partialOrd", Glue, ""),
    entry("partiallyOrdered", Glue, ""),
    entry("partition", Model(Unproven(Why::Consumers), &[]), ""),
    entry("partitionPoint", Model(Unproven(Why::Collections), &[]), ""),
    entry("pathJoin", Model(Unproven(Why::Collections), &[]), ""),
    entry("pathSetExtension", Model(Unproven(Why::Outside), &[Row("pathExtension")]), "its rows of `$stdSites`, std's sites of its function's panics").held(Net::StdSites),
    entry("peek", Model(Unproven(Why::Memory), &[Path("core::iter::Peekable::peek")]), "ADR 0561: `Peekable::peek`, whose body reaches memory"),
    entry("peekMut", Model(Unproven(Why::Collections), &[]), ""),
    entry("peekMutDrop", Model(Unproven(Why::Collections), &[]), ""),
    entry("peekMutTop", Model(Unproven(Why::Collections), &[]), ""),
    entry("peekSome", Model(Unproven(Why::Memory), &[]), "ADRs 0561, 0051: `$peek`'s of items that may look like `None`, each boxed, an `Option`'s items (#1826)"),
    entry("pop", Model(Unproven(Why::Collections), &[]), ""),
    entry("popFrontIf", Model(Unproven(Why::Collections), &[]), ""),
    entry("popIf", Model(Unproven(Why::Collections), &[]), ""),
    entry("position", Model(Unproven(Why::Consumers), &[]), ""),
    entry("pushFrontMut", Model(Unproven(Why::Collections), &[]), ""),
    entry("pushMut", Model(Unproven(Why::Collections), &[]), ""),
    entry("range", Model(Pitted, &[Path("core::ops::Range"), Row("vecFromTrustedLen")]), ""),
    entry("rangeCountOut", Model(Unproven(Why::Ranges), &[]), ""),
    entry("rangeFirst", Model(Unproven(Why::Ranges), &[]), ""),
    entry("rangeFromNext", Model(Unproven(Why::Ranges), &[Row("stepForward")]), "its rows of `$stdSites`, std's sites of its function's panics"),
    entry("rangeLast", Model(Unproven(Why::Ranges), &[]), ""),
    entry("rangeLastOut", Model(Unproven(Why::Ranges), &[]), ""),
    entry("rangeLen", Model(Pitted, &[Path("core::ops::Range")]), ""),
    entry("rangeNext", Model(Unproven(Why::Ranges), &[Path("core::ops::Range")]), ""),
    entry("rangeNextBack", Model(Pitted, &[Path("core::ops::Range")]), ""),
    entry("rangeNth", Model(Unproven(Why::Ranges), &[Path("core::ops::Range")]), ""),
    entry("rangeNthBack", Model(Unproven(Why::Ranges), &[Path("core::ops::Range")]), ""),
    entry("rcClone", Glue, ""),
    entry("rcDrop", Glue, ""),
    entry("rchunksMut", Model(Unproven(Why::Collections), &[]), ""),
    entry("reduce", Model(Unproven(Why::Consumers), &[]), ""),
    entry("refCellEq", Form(&[Row("refCellEq")]), "the lowering's, std_impls.rs"),
    entry("refCellSwap", Model(Unproven(Why::Cells), &[Row("refCellSwap")]), "its rows of `$stdSites`, std's sites of its function's panics, its row read by the lowering's std_types/cell.rs"),
    entry("remove", Model(Unproven(Why::Collections), &[]), ""),
    entry("removeEntry", Model(Unproven(Why::Collections), &[]), ""),
    entry("removeItem", Model(Unproven(Why::Collections), &[]), ""),
    entry("removeOpt", Model(Unproven(Why::Collections), &[]), ""),
    entry("removeOutOfBounds", Model(Unproven(Why::Collections), &[]), ""),
    entry("repeat", Model(Unproven(Why::Collections), &[]), "`vec![item; count]`, `Vec::from_elem`"),
    entry("repeatItems", Model(Unproven(Why::Outside), &[Row("sliceRepeat")]), "its rows of `$stdSites`, std's sites of its function's panics").held(Net::StdSites),
    entry("repeatText", Model(Unproven(Why::Outside), &[Row("sliceRepeat")]), "its rows of `$stdSites`, std's sites of its function's panics").held(Net::StdSites),
    entry("repeating", Model(Pitted, &[Path("core::iter::Repeat")]), ""),
    entry("repeatingN", Model(Pitted, &[Path("core::iter::RepeatN")]), ""),
    entry("replaceRange", Model(Unproven(Why::Collections), &[]), ""),
    entry("replaceWith", Model(Unproven(Why::Collections), &[]), ""),
    entry("reserve", Model(Unproven(Why::Collections), &[]), ""),
    entry("reserveHint", Model(Unproven(Why::Collections), &[]), ""),
    entry("resize", Model(Unproven(Why::Collections), &[]), ""),
    entry("resizeWith", Model(Unproven(Why::Collections), &[]), ""),
    entry("retain", Model(Unproven(Why::Collections), &[]), ""),
    entry("retainBack", Model(Unproven(Why::Collections), &[]), ""),
    entry("retainDeque", Model(Unproven(Why::Collections), &[]), ""),
    entry("retainMap", Model(Unproven(Why::Collections), &[]), ""),
    entry("retainMut", Model(Unproven(Why::Collections), &[]), ""),
    entry("retainSet", Model(Unproven(Why::Collections), &[]), ""),
    entry("retainText", Model(Unproven(Why::Collections), &[]), ""),
    entry("rev", Model(Pitted, &[Path("core::iter::Rev")]), ""),
    entry("roomFor", Model(Unproven(Why::Collections), &[Row("capacityOverflow")]), ""),
    entry("roomForCounted", Model(Unproven(Why::Collections), &[]), ""),
    entry("roomForHint", Model(Unproven(Why::Collections), &[Row("capacityOverflow"), Row("vecFromTrustedLen")]), ""),
    entry("rotateLeft", Model(Unproven(Why::Collections), &[Row("rotateLeft"), Row("vecDequeRotateLeft")]), ""),
    entry("rotateRight", Model(Unproven(Why::Collections), &[Row("rotateRight"), Row("vecDequeRotateRight")]), ""),
    entry("seekDefaults", Model(Unproven(Why::Outside), &[Row("byteSliceConsume"), Row("vecDequeDrain")]), "its rows of `$stdSites`, std's sites of its function's panics").held(Net::StdSites),
    entry("selectNthUnstable", Model(Unproven(Why::Sort), &[Row("selectNth")]), ""),
    entry("setReplace", Model(Unproven(Why::Collections), &[]), ""),
    entry("setTake", Model(Unproven(Why::Collections), &[]), ""),
    entry("shift", Model(Unproven(Why::Collections), &[]), ""),
    entry("showBorrowed", Model(Unproven(Why::Format), &[]), ""),
    entry("sink", Model(Unproven(Why::Format), &[]), ""),
    entry("sizeHint", Glue, ""),
    entry("skipping", Model(Pitted, &[Path("core::iter::Skip"), Row("skipCount"), Row("skipLast")]), ""),
    entry("slice", Model(Unproven(Why::Collections), &[]), ""),
    entry("sliceSplitAt", Model(Unproven(Why::Collections), &[]), ""),
    entry("slicing", Glue, ""),
    entry("some", Model(Pitted, &[]), ""),
    entry("someGet", Glue, ""),
    entry("someItems", Model(Pitted, &[]), ""),
    entry("someMut", Glue, ""),
    entry("someValue", Model(Pitted, &[]), ""),
    entry("somes", Model(Pitted, &[]), ""),
    entry("sortByCachedKey", Model(Unproven(Why::Sort), &[]), ""),
    entry("sortStable", Model(Unproven(Why::Sort), &[]), ""),
    entry("sortUnstable", Model(Unproven(Why::Sort), &[]), ""),
    entry("sortedEntries", Model(Unproven(Why::Collections), &[]), ""),
    entry("sortedKeys", Model(Unproven(Why::Collections), &[]), ""),
    entry("splice", Model(Unproven(Why::Collections), &[]), ""),
    entry("splicing", Model(Unproven(Why::Collections), &[]), ""),
    entry("splitAt", Model(Unproven(Why::Outside), &[Row("strSplitAt")]), "its rows of `$stdSites`, std's sites of its function's panics").held(Net::StdSites),
    entry("splitAtMut", Model(Unproven(Why::Collections), &[]), ""),
    entry("splitChunkMut", Model(Unproven(Why::Collections), &[]), ""),
    entry("splitEndMut", Model(Unproven(Why::Collections), &[]), ""),
    entry("splitOff", Model(Unproven(Why::Collections), &[]), ""),
    entry("stdSites", Glue, ""),
    entry("step", Model(Unproven(Why::Outside), &[Row("stepBy")]), "its rows of `$stdSites`, std's sites of its function's panics").held(Net::StdSites),
    entry("stepBy", Model(Pitted, &[Path("core::iter::StepBy")]), ""),
    entry("stepByEnds", Model(Pitted, &[Path("core::iter::StepBy")]), ""),
    entry("stepsOf", Model(Unproven(Why::JsOwn), &[]), "ADR 0570, amended (#2039): a type parameter's value std's compiled body is given, as it is, never armed to close at its end"),
    entry("strDrain", Model(Unproven(Why::Collections), &[]), ""),
    entry("strExtendWithin", Model(Unproven(Why::Collections), &[]), ""),
    entry("strPop", Model(Unproven(Why::Collections), &[]), ""),
    entry("strRemove", Model(Unproven(Why::Collections), &[]), ""),
    entry("strSplitAtMut", Model(Unproven(Why::Outside), &[Row("strSplitAtMut")]), "its rows of `$stdSites`, std's sites of its function's panics").held(Net::StdSites),
    entry("strSplitOff", Model(Unproven(Why::Collections), &[]), ""),
    entry("strTruncate", Model(Unproven(Why::Collections), &[]), ""),
    entry("stringPanic", Glue, ""),
    entry("swap", Model(Unproven(Why::Collections), &[Row("vecDequeSwap")]), ""),
    entry("swapRemove", Model(Unproven(Why::Collections), &[Row("swapRemove")]), ""),
    entry("swapWithSlice", Model(Unproven(Why::Collections), &[]), ""),
    entry("tableRoomFor", Model(Unproven(Why::Outside), &[Row("hashTableCapacityOverflow")]), "its rows of `$stdSites`, std's sites of its function's panics").held(Net::StdSites),
    entry("take", Model(Unproven(Why::Collections), &[]), ""),
    entry("taking", Model(Pitted, &[Path("core::iter::Take")]), ""),
    entry("took", Glue, ""),
    entry("traitImpl", Glue, ""),
    entry("treeAppend", Model(Unproven(Why::Collections), &[]), ""),
    entry("treeEnd", Model(Unproven(Why::Collections), &[]), ""),
    entry("treeFrom", Model(Unproven(Why::Collections), &[]), ""),
    entry("treeFromSorted", Model(Unproven(Why::Collections), &[]), ""),
    entry("treeRange", Model(Unproven(Why::Collections), &[Row("btreeRange")]), ""),
    entry("treeSearchBound", Model(Unproven(Why::Outside), &[Row("btreeRange")]), "its rows of `$stdSites`, std's sites of its function's panics").held(Net::StdSites),
    entry("treeSplitOff", Model(Unproven(Why::Collections), &[]), ""),
    entry("truncate", Model(Unproven(Why::Collections), &[]), ""),
    entry("trustedZip", Model(Pitted, &[Path("core::iter::Zip")]), ""),
    entry("trustedZipEnds", Model(Pitted, &[Path("core::iter::Zip")]), ""),
    entry("tryFoldOk", Model(Unproven(Why::Consumers), &[]), ""),
    entry("tryOf", Model(Pitted, &[]), ""),
    entry("tupleClone", Form(&[Row("tupleClone")]), "the lowering's, std_impls/clones.rs"),
    entry("unborrow", Model(Unproven(Why::Cells), &[]), ""),
    entry("unfused", Model(Pitted, &[Path("core::iter::Fuse")]), ""),
    entry("unfusedUnless", Model(Pitted, &[Path("core::iter::Fuse")]), ""),
    entry("untracked", Model(Pitted, &[]), ""),
    entry("unwinds", Glue, ""),
    entry("unwindsOn", Glue, ""),
    entry("unwound", Glue, ""),
    entry("unwrap", Glue, ""),
    entry("unwrapOk", Glue, ""),
    entry("unzip", Model(Unproven(Why::Consumers), &[]), ""),
    entry("upgrade", Glue, ""),
    entry("vecDequeRange", Form(&[Row("vecDequeRange")]), "the lowering's, std_calls.rs"),
    entry("vecDequeSplitOff", Form(&[Row("vecDequeSplitOff")]), "the lowering's, combinators.rs"),
    entry("vecExtractIf", Form(&[Row("vecExtractIf")]), "the lowering's, std_types/slice.rs"),
    entry("view", Glue, ""),
    entry("viewGet", Glue, ""),
    entry("viewRange", Glue, ""),
    entry("withCapacity", Model(Unproven(Why::Collections), &[]), ""),
    entry("writeCloneOfSlice", Model(Unproven(Why::Outside), &[Row("writeCloneOfSlice")]), "its rows of `$stdSites`, std's sites of its function's panics").held(Net::StdSites),
    entry("writeCopyOfSlice", Form(&[Row("writeCopyOfSlice")]), "the lowering's, std_types/slice.rs"),
    entry("zipEnds", Model(Pitted, &[Path("core::iter::Zip")]), ""),
];

/// The catalog as JSON, which `rust-js --std-models` prints: each helper,
/// whether it's glue, its proof and reason, the std items it stands for,
/// its note and the net that holds it; and each reason's text.
pub fn json() -> String {
    let entries: Vec<serde_json::Value> = MODELS
        .iter()
        .map(|entry| {
            let (kind, pitted, why, stands): (&str, bool, Option<&str>, &[Stands]) = match entry.kind {
                Glue => ("glue", false, Some("glue"), &[]),
                Model(Pitted, stands) => ("model", true, None, stands),
                Model(Unproven(why), stands) => ("model", false, Some(why.name()), stands),
                Form(stands) => ("form", false, None, stands),
            };
            let stands_for: Vec<String> = (stands.iter())
                .map(|stands| match *stands {
                    Path(path) => path.to_string(),
                    Row(row) => format!("$stdSites.{row}"),
                })
                .collect();
            // A helper the runtime exports by `$` and its name; a form by its
            // row's.
            let name = match entry.kind {
                Form(_) => entry.helper.to_string(),
                _ => format!("${}", entry.helper),
            };
            serde_json::json!({
                "helper": name,
                "kind": kind,
                "pitted": pitted,
                "why": why,
                "standsFor": stands_for,
                "note": entry.note,
                "heldBy": entry.held.name(),
            })
        })
        .collect();
    let whys: serde_json::Map<String, serde_json::Value> = WHYS
        .iter()
        .map(|why| (why.name().to_string(), serde_json::Value::from(why.says())))
        .chain([("glue".to_string(), serde_json::Value::from(GLUE))])
        .collect();
    serde_json::to_string_pretty(&serde_json::json!({ "models": entries, "why": whys })).expect("the catalog is JSON")
}

/// Every reason, in order.
const WHYS: &[Why] = &[
    Why::Outside,
    Why::Consumers,
    Why::Iterators,
    Why::JsOwn,
    Why::Memory,
    Why::Ranges,
    Why::Collections,
    Why::Sort,
    Why::Format,
    Why::Cells,
    Why::Numbers,
];

/// Each std item a model stands for that names nothing in this
/// toolchain's std: a path through its public modules, `core::iter::Rev`,
/// its last part a trait's or a type's method where the rest names one,
/// `core::iter::DoubleEndedIterator::rfold`. Asked where `bun run std`
/// reads std, which fails on one (`compiled_std::raised`).
pub(crate) fn unresolved(tcx: TyCtxt<'_>) -> Vec<String> {
    let mut missing = Vec::new();
    for entry in MODELS {
        let (Model(_, stands) | Form(stands)) = entry.kind else {
            continue;
        };
        for stands in stands {
            let Path(path) = *stands else { continue };
            if resolved(tcx, path).is_none() {
                missing.push(format!("${}: {path}", entry.helper));
            }
        }
    }
    missing
}

/// The item `path` names in std, or a method of the item its parent names.
pub(crate) fn resolved(tcx: TyCtxt<'_>, path: &str) -> Option<DefId> {
    let mut parts: Vec<&str> = path.split("::").collect();
    let krate = match parts.remove(0) {
        "core" => super::StdCrate::Core,
        "alloc" => super::StdCrate::Alloc,
        "std" => super::StdCrate::Std,
        _ => return None,
    };
    if let Some(id) = super::registry::by_path(tcx, krate, &parts) {
        return Some(id);
    }
    let (method, parent) = parts.split_last()?;
    let (kind, owner) = super::registry::by_path_kind(tcx, krate, parent)?;
    let items: Vec<DefId> = match kind {
        DefKind::Trait => tcx.associated_item_def_ids(owner).to_vec(),
        DefKind::Struct | DefKind::Enum | DefKind::Union => tcx
            .inherent_impls(owner)
            .iter()
            .flat_map(|&imp| tcx.associated_item_def_ids(imp).iter().copied())
            .collect(),
        _ => return None,
    };
    items.into_iter().find(|&item| tcx.item_name(item).as_str() == *method)
}

/// Each row of `$stdSites` a model or a form says it reads that `written`,
/// the rows the std build writes, lacks: of a catalog that names a row
/// nothing writes. Asked where `bun run std` has written them
/// (`compiled_std::raised`).
pub(crate) fn rows_unwritten(written: &dyn Fn(&str) -> bool) -> Vec<String> {
    let mut missing = Vec::new();
    for entry in MODELS {
        let (Model(_, stands) | Form(stands)) = entry.kind else {
            continue;
        };
        for stands in stands {
            if let Row(row) = *stands
                && !written(row)
            {
                missing.push(format!("{}: {row}", entry.helper));
            }
        }
    }
    missing
}
