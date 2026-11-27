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

pub mod rows;

use rows::StdRow;

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
    /// a range's `Iterator` std's build doesn't compile yet: an `a..`'s, next (ADR 1206's S4), and an `a..=b`'s, whose `exhausted` field rust-js's range doesn't hold (#1589)
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
                "a range's `Iterator` std's build doesn't compile yet: an `a..`'s, next (ADR 1206's S4), and an `a..=b`'s, whose `exhausted` field rust-js's range doesn't hold (#1589)"
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
    Row(StdRow),
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

/// One of the runtime's helpers, by its name, the export `$` and it, what
/// it is, and a note, its ADR.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Entry {
    pub helper: &'static str,
    pub kind: Kind,
    pub note: &'static str,
}

use Kind::{Form, Glue, Model};
use Proof::{Pitted, Unproven};
use Stands::{Path, Row};

const fn entry(helper: &'static str, kind: Kind, note: &'static str) -> Entry {
    Entry { helper, kind, note }
}

/// The catalog, by helper, each on one line, which the twin net's bless
/// writes (`test/twin-net.ts`' `blessedCatalog`).
#[rustfmt::skip]
pub const MODELS: &[Entry] = &[
    entry("BTreeMap", Model(Unproven(Why::Collections), &[]), ""),
    entry("BTreeSet", Model(Unproven(Why::Collections), &[]), ""),
    entry("KeyMap", Model(Unproven(Why::Collections), &[]), ""),
    entry("KeySet", Model(Unproven(Why::Collections), &[]), ""),
    entry("abort", Model(Unproven(Why::Outside), &[Row(StdRow::PanicNounwindNobacktrace)]), "its rows of `$stdSites`, std's sites of its function's panics"),
    entry("add", Model(Unproven(Why::Collections), &[]), ""),
    entry("advanceSlice", Model(Unproven(Why::JsOwn), &[]), "ADRs 0422, 0465: an array's iterator an adapter holds, advanced by index"),
    entry("andModify", Model(Unproven(Why::Collections), &[]), ""),
    entry("append", Model(Unproven(Why::Collections), &[]), ""),
    entry("arcFromTrustedLen", Form(&[Row(StdRow::ArcFromTrustedLen)]), "the lowering's, iterators/mir_steps/collect.rs"),
    entry("arcSliceLayout", Form(&[Row(StdRow::ArcSliceLayout)]), "the lowering's, iterators/mir_steps/collect.rs"),
    entry("asChunksMut", Model(Unproven(Why::Collections), &[]), ""),
    entry("asciiCase", Model(Unproven(Why::Collections), &[]), ""),
    entry("assign", Glue, ""),
    entry("at", Glue, ""),
    entry("bidirectionalMerge", Model(Unproven(Why::Sort), &[Row(StdRow::SortTotalOrder)]), "its rows of `$stdSites`, std's sites of its function's panics"),
    entry("binarySearchBy", Model(Unproven(Why::Collections), &[]), ""),
    entry("borrow", Model(Unproven(Why::Cells), &[Path("core::cell::RefCell::borrow")]), ""),
    entry("borrowMut", Model(Unproven(Why::Cells), &[Path("core::cell::RefCell::borrow_mut")]), ""),
    entry("boxLast", Form(&[Row(StdRow::BoxLast)]), "the lowering's, recognition/own_steps.rs, std_calls.rs's frame of a step"),
    entry("btreeSet", Model(Unproven(Why::Collections), &[]), ""),
    entry("bufReader", Model(Unproven(Why::Outside), &[Row(StdRow::BufReaderStreamPosition)]), "its rows of `$stdSites`, std's sites of its function's panics"),
    entry("byteLen", Model(Unproven(Why::Collections), &[]), ""),
    entry("cannotUnwind", Model(Unproven(Why::Outside), &[Row(StdRow::PanicNounwind)]), "its rows of `$stdSites`, std's sites of its function's panics"),
    entry("capacity", Model(Unproven(Why::Collections), &[]), ""),
    entry("catchUnwind", Glue, ""),
    entry("catchable", Glue, ""),
    entry("chars", Model(Unproven(Why::JsOwn), &[Path("core::str::Chars")]), "ADR 1117: a string's `Chars`, the runtime's lazy one"),
    entry("checkedSplitAtMut", Model(Unproven(Why::Collections), &[]), ""),
    entry("chunksExactMut", Model(Unproven(Why::Collections), &[]), ""),
    entry("chunksMut", Model(Unproven(Why::Collections), &[]), ""),
    entry("clampBy", Model(Unproven(Why::Collections), &[]), ""),
    entry("clampFloat", Model(Unproven(Why::Outside), &[Row(StdRow::Clamp)]), "its rows of `$stdSites`, std's sites of its function's panics"),
    entry("cleanup", Glue, ""),
    entry("clone", Model(Pitted, &[]), ""),
    entry("cloneFromSlice", Model(Unproven(Why::Collections), &[]), ""),
    entry("cloneIntoVec", Model(Unproven(Why::Collections), &[]), ""),
    entry("cloneOfClass", Model(Unproven(Why::Collections), &[]), ""),
    entry("cloned", Form(&[Path("core::iter::adapters::cloned::Cloned")]), "the lowering's, iterators/mir_steps/copies.rs: a `cloned()`, a map of each item's clone"),
    entry("clonedAt", Form(&[Row(StdRow::ClonedAt)]), "the lowering's, std_impls/clones.rs: each std type's key is held to std's MIR by test/std-cloned-at.test.ts, not a case each; a few corpus cases cover its reader (Campaign 5 step 2c)"),
    entry("closeBare", Glue, ""),
    entry("closeHolding", Glue, ""),
    entry("closing", Glue, ""),
    entry("cmp", Glue, ""),
    entry("cmpItems", Model(Unproven(Why::Consumers), &[]), ""),
    entry("collectDropping", Model(Pitted, &[Path("core::iter::Iterator::collect")]), ""),
    entry("collectOptions", Model(Unproven(Why::Consumers), &[]), ""),
    entry("collectResults", Model(Unproven(Why::Consumers), &[]), ""),
    entry("collectVec", Model(Unproven(Why::Consumers), &[]), ""),
    entry("collectsInPlace", Model(Unproven(Why::Consumers), &[]), ""),
    entry("consume", Model(Pitted, &[]), ""),
    entry("copied", Model(Unproven(Why::Outside), &[Row(StdRow::CopiedCount), Row(StdRow::CopiedLast)]), "its rows of `$stdSites`, std's sites of its function's panics"),
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
    entry("dequeInsert", Model(Unproven(Why::Collections), &[Row(StdRow::VecDequeInsert)]), ""),
    entry("dequeSwapRemove", Model(Unproven(Why::Collections), &[]), ""),
    entry("dispose", Glue, ""),
    entry("disposeIterator", Model(Unproven(Why::JsOwn), &[]), "ADR 0140, ADR 1136: a `dyn Iterator`'s JS iterator, a binding's, closed as JS's own"),
    entry("drain", Model(Unproven(Why::Collections), &[]), ""),
    entry("drainAll", Model(Unproven(Why::Collections), &[]), ""),
    entry("dropEach", Glue, ""),
    entry("dropItems", Glue, ""),
    entry("dropping", Glue, ""),
    entry("droppingIterator", Model(Unproven(Why::JsOwn), &[]), "ADR 0140, ADR 1136: a `dyn Iterator`'s JS iterator stepped dropping what it gives, as JS's own"),
    entry("durationAdd", Model(Unproven(Why::Outside), &[Row(StdRow::DurationAdd)]), "its rows of `$stdSites`, std's sites of its function's panics"),
    entry("durationNew", Model(Unproven(Why::Outside), &[Row(StdRow::DurationNew)]), "its rows of `$stdSites`, std's sites of its function's panics"),
    entry("durationSub", Model(Unproven(Why::Outside), &[Row(StdRow::DurationSub)]), "its rows of `$stdSites`, std's sites of its function's panics"),
    entry("encodeCharUtf16", Model(Unproven(Why::Outside), &[Row(StdRow::EncodeUtf16)]), "its rows of `$stdSites`, std's sites of its function's panics"),
    entry("encodeUtf8", Model(Unproven(Why::Outside), &[Row(StdRow::EncodeUtf8)]), "its rows of `$stdSites`, std's sites of its function's panics"),
    entry("endEntry", Model(Unproven(Why::Collections), &[]), ""),
    entry("entry", Model(Unproven(Why::Collections), &[]), ""),
    entry("entryGet", Model(Unproven(Why::Collections), &[]), ""),
    entry("entryInsert", Model(Unproven(Why::Collections), &[]), ""),
    entry("entryOrInsert", Model(Unproven(Why::Collections), &[]), ""),
    entry("entryOrInsertWith", Model(Unproven(Why::Collections), &[]), ""),
    entry("entryRemove", Model(Unproven(Why::Collections), &[]), ""),
    entry("enumerate", Model(Pitted, &[Path("core::iter::Enumerate"), Row(StdRow::EnumerateCount)]), ""),
    entry("eq", Glue, ""),
    entry("exactLen", Model(Pitted, &[Path("core::iter::ExactSizeIterator::len"), Row(StdRow::ExactSizeLen)]), ""),
    entry("exchange", Glue, ""),
    entry("extend", Model(Unproven(Why::Collections), &[Row(StdRow::VecExtendTrusted)]), ""),
    entry("extendFromSlice", Form(&[Path("alloc::vec::Vec::extend_from_slice")]), "the lowering's, combinators.rs: each item pushed, a clone"),
    entry("extendMap", Model(Unproven(Why::Collections), &[]), ""),
    entry("extendText", Model(Unproven(Why::Collections), &[]), ""),
    entry("extractIf", Model(Unproven(Why::Collections), &[]), ""),
    entry("fill", Model(Unproven(Why::Collections), &[Path("core::slice::<[T]>::fill")]), ""),
    entry("fillWith", Model(Unproven(Why::Collections), &[]), ""),
    entry("filterDropping", Model(Pitted, &[Path("core::iter::Filter")]), ""),
    entry("filterEnds", Model(Pitted, &[Path("core::iter::Filter")]), ""),
    entry("findBack", Model(Pitted, &[Path("core::iter::DoubleEndedIterator::rfind")]), ""),
    entry("findDropping", Model(Unproven(Why::Consumers), &[]), ""),
    entry("flatLast", Model(Unproven(Why::Outside), &[Row(StdRow::FlattenLast)]), "its rows of `$stdSites`, std's sites of its function's panics"),
    entry("flatMap", Model(Pitted, &[Path("core::iter::FlatMap"), Row(StdRow::FlattenCount)]), ""),
    entry("flattenEnds", Model(Pitted, &[Path("core::iter::Flatten")]), ""),
    entry("fmtOrPanic", Model(Unproven(Why::Outside), &[Row(StdRow::Format), Row(StdRow::ToString)]), "its row by the name format_args.rs gives it, `$stdSites[kind]`, `format!`'s or `to_string()`'s"),
    entry("fmtPrintFailed", Model(Unproven(Why::Outside), &[Row(StdRow::PrintFailed)]), "its rows of `$stdSites`, std's sites of its function's panics"),
    entry("foldBack", Model(Pitted, &[Path("core::iter::DoubleEndedIterator::rfold")]), ""),
    entry("foldDropping", Model(Unproven(Why::Consumers), &[]), ""),
    entry("foldOf", Model(Unproven(Why::Consumers), &[]), ""),
    entry("foldToArray", Model(Unproven(Why::Consumers), &[]), ""),
    entry("folded", Model(Unproven(Why::Consumers), &[]), ""),
    entry("force", Model(Unproven(Why::Outside), &[Row(StdRow::LazyCellPoisoned)]), "its rows of `$stdSites`, std's sites of its function's panics"),
    entry("formatting", Model(Unproven(Why::Format), &[]), ""),
    entry("fromDigit", Model(Unproven(Why::Outside), &[Row(StdRow::FromDigit)]), "its rows of `$stdSites`, std's sites of its function's panics"),
    entry("getDisjointMut", Model(Unproven(Why::Collections), &[]), ""),
    entry("getOrInit", Model(Unproven(Why::Outside), &[Row(StdRow::OnceCellReentrant)]), "its rows of `$stdSites`, std's sites of its function's panics"),
    entry("given", Glue, ""),
    entry("givenDrop", Glue, ""),
    entry("grew", Model(Pitted, &[]), ""),
    entry("grownBy", Model(Unproven(Why::Collections), &[Row(StdRow::CapacityOverflow), Row(StdRow::VecDequeExtendTrusted), Row(StdRow::VecDequeReserve), Row(StdRow::VecExtendTrusted)]), ""),
    entry("grownVec", Model(Pitted, &[]), ""),
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
    entry("keepCapacity", Model(Pitted, &[]), ""),
    entry("knowing", Model(Pitted, &[]), ""),
    entry("last", Model(Unproven(Why::Consumers), &[]), ""),
    entry("lastDropping", Model(Unproven(Why::Consumers), &[]), ""),
    entry("lazyChain", Model(Pitted, &[Path("core::iter::Chain"), Row(StdRow::ChainCount), Row(StdRow::FnOnceCall)]), ""),
    entry("lazyChunkBy", Model(Unproven(Why::Memory), &[Path("core::slice::ChunkBy")]), "its `Iterator::next` steps the slice's `windows`, which reach memory, `NonNull<[T]>`: proved through std's build once it models a slice's pointers (#1458)"),
    entry("lazyChunkByMut", Model(Unproven(Why::Memory), &[Path("core::slice::ChunkByMut")]), "its `Iterator::next` steps the slice's `windows`, which reach memory, `NonNull<[T]>`: proved through std's build once it models a slice's pointers (#1458)"),
    entry("lazyLockPoisoned", Model(Unproven(Why::Outside), &[Row(StdRow::LazyLockPoisoned)]), "its rows of `$stdSites`, std's sites of its function's panics"),
    entry("lazyMapWhile", Model(Pitted, &[Path("core::iter::MapWhile")]), ""),
    entry("lazyRange", Model(Pitted, &[Path("core::ops::Range")]), ""),
    entry("lazyScan", Model(Pitted, &[Path("core::iter::Scan")]), ""),
    entry("lazySkipWhile", Model(Pitted, &[Path("core::iter::SkipWhile")]), ""),
    entry("lazySliceSplitBy", Model(Unproven(Why::Memory), &[Path("core::slice::Split")]), "its `Iterator::next` searches the slice by its pointers, which reach memory: proved through std's build once it models a slice's pointers (#1458)"),
    entry("lazySliceSplitByMut", Model(Unproven(Why::Memory), &[Path("core::slice::SplitMut")]), "its `Iterator::next` searches the slice by its pointers, which reach memory: proved through std's build once it models a slice's pointers (#1458)"),
    entry("lazyTakeWhile", Model(Pitted, &[Path("core::iter::TakeWhile")]), ""),
    entry("lazyZip", Model(Pitted, &[Path("core::iter::Zip")]), ""),
    entry("lent", Model(Pitted, &[Row(StdRow::BoxLast)]), ""),
    entry("live", Glue, ""),
    entry("localKeyReplace", Form(&[Row(StdRow::LocalKeyReplace)]), "the lowering's, std_types/cell.rs"),
    entry("localKeySet", Form(&[Row(StdRow::LocalKeySet)]), "the lowering's, std_types/cell.rs"),
    entry("localKeyWithBorrow", Form(&[Row(StdRow::LocalKeyWithBorrow)]), "the lowering's, std_types/cell.rs"),
    entry("localKeyWithBorrowMut", Form(&[Row(StdRow::LocalKeyWithBorrowMut)]), "the lowering's, std_types/cell.rs"),
    entry("madeEach", Glue, ""),
    entry("madeInTurn", Glue, ""),
    entry("main", Glue, "ADR 1198: a binary's start, which every case's binary imports"),
    entry("makeMut", Glue, ""),
    entry("mapEnds", Model(Pitted, &[Path("core::iter::Map")]), ""),
    entry("mapExtractIf", Model(Unproven(Why::Collections), &[]), ""),
    entry("mapFrom", Model(Unproven(Why::Collections), &[]), ""),
    entry("mapGetDisjointMut", Model(Unproven(Why::Collections), &[Row(StdRow::MapGetDisjointMut)]), "its row read by the lowering's std_types/map.rs"),
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
    entry("nextMultipleOf", Model(Unproven(Why::Outside), &[Row(StdRow::NextMultipleOf)]), "its rows of `$stdSites`, std's sites of its function's panics"),
    entry("nextSome", Model(Unproven(Why::JsOwn), &[]), "ADRs 0140, 0051: the step of any JS iterator, its item boxed where it may look like `None`"),
    entry("nth", Model(Unproven(Why::Consumers), &[]), ""),
    entry("nthBack", Model(Pitted, &[]), ""),
    entry("nthDropping", Model(Unproven(Why::Consumers), &[]), ""),
    entry("optionCloned", Form(&[Path("core::option::Option::cloned")]), "the lowering's, std_types/option.rs: `map(T::clone)`"),
    entry("orInsert", Model(Unproven(Why::Collections), &[]), ""),
    entry("orInsertWith", Model(Unproven(Why::Collections), &[]), ""),
    entry("orInsertWithKey", Model(Unproven(Why::Collections), &[]), ""),
    entry("ord", Glue, ""),
    entry("ordClamp", Form(&[Row(StdRow::OrdClamp)]), "the lowering's, ordering.rs"),
    entry("ordTreeMap", Model(Unproven(Why::Collections), &[]), ""),
    entry("ordTreeSet", Model(Unproven(Why::Collections), &[]), ""),
    entry("out", Model(Unproven(Why::Format), &[]), ""),
    entry("panic", Glue, ""),
    entry("panicking", Glue, ""),
    entry("parseBig", Model(Unproven(Why::Numbers), &[Row(StdRow::FromStrRadix)]), "its rows of `$stdSites`, std's sites of its function's panics"),
    entry("parseInt", Model(Unproven(Why::Numbers), &[Row(StdRow::FromStrRadix)]), ""),
    entry("partialEq", Glue, ""),
    entry("partialOrd", Glue, ""),
    entry("partiallyOrdered", Glue, ""),
    entry("partition", Model(Unproven(Why::Consumers), &[]), ""),
    entry("partitionPoint", Model(Unproven(Why::Collections), &[]), ""),
    entry("pathJoin", Model(Unproven(Why::Collections), &[]), ""),
    entry("pathSetExtension", Model(Unproven(Why::Outside), &[Row(StdRow::PathExtension)]), "its rows of `$stdSites`, std's sites of its function's panics"),
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
    entry("range", Model(Pitted, &[Path("core::ops::Range"), Row(StdRow::VecFromTrustedLen)]), ""),
    entry("rangeCountOut", Model(Pitted, &[]), ""),
    entry("rangeFirst", Model(Pitted, &[]), ""),
    entry("rangeFromNext", Model(Unproven(Why::Ranges), &[Row(StdRow::StepForward)]), "its rows of `$stdSites`, std's sites of its function's panics"),
    entry("rangeLast", Model(Pitted, &[]), ""),
    entry("rangeLastOut", Model(Pitted, &[]), ""),
    entry("rangeLen", Model(Pitted, &[Path("core::ops::Range")]), ""),
    entry("rangeNext", Model(Pitted, &[Path("core::ops::Range")]), ""),
    entry("rangeNextBack", Model(Pitted, &[Path("core::ops::Range")]), ""),
    entry("rangeNth", Model(Pitted, &[Path("core::ops::Range")]), "pitted against std's `next`, the default `nth` over it, not std's own `nth`, which never runs (#2604)"),
    entry("rangeNthBack", Model(Pitted, &[Path("core::ops::Range")]), "pitted against std's `next_back`, the default `nth_back` over it, not std's own `nth_back`, which never runs (#2604)"),
    entry("rcClone", Glue, ""),
    entry("rcDrop", Glue, ""),
    entry("rcFromTrustedLen", Form(&[Row(StdRow::RcFromTrustedLen)]), "the lowering's, iterators/mir_steps/collect.rs"),
    entry("rcSliceLayout", Form(&[Row(StdRow::RcSliceLayout)]), "the lowering's, iterators/mir_steps/collect.rs"),
    entry("rchunksMut", Model(Unproven(Why::Collections), &[]), ""),
    entry("reduce", Model(Unproven(Why::Consumers), &[]), ""),
    entry("refCellEq", Form(&[Row(StdRow::RefCellEq)]), "the lowering's, std_impls.rs"),
    entry("refCellSwap", Model(Unproven(Why::Cells), &[Row(StdRow::RefCellSwap)]), "its rows of `$stdSites`, std's sites of its function's panics, its row read by the lowering's std_types/cell.rs"),
    entry("refCellTake", Form(&[Row(StdRow::RefCellTake)]), "the lowering's, std_types/cell.rs"),
    entry("remove", Model(Unproven(Why::Collections), &[]), ""),
    entry("removeEntry", Model(Unproven(Why::Collections), &[]), ""),
    entry("removeItem", Model(Unproven(Why::Collections), &[]), ""),
    entry("removeOpt", Model(Unproven(Why::Collections), &[]), ""),
    entry("removeOutOfBounds", Model(Unproven(Why::Collections), &[]), ""),
    entry("repeat", Model(Unproven(Why::Collections), &[Path("alloc::vec::from_elem")]), "`vec![item; count]`, `Vec::from_elem`"),
    entry("repeatItems", Model(Unproven(Why::Outside), &[Row(StdRow::SliceRepeat)]), "its rows of `$stdSites`, std's sites of its function's panics"),
    entry("repeatText", Model(Unproven(Why::Outside), &[Row(StdRow::SliceRepeat)]), "its rows of `$stdSites`, std's sites of its function's panics"),
    entry("repeating", Model(Pitted, &[Path("core::iter::Repeat"), Row(StdRow::ClonedAt)]), "each step's clone at its row of `clonedAt`"),
    entry("repeatingN", Model(Pitted, &[Path("core::iter::RepeatN"), Row(StdRow::ClonedAt)]), "each step's clone at its row of `clonedAt`"),
    entry("replaceRange", Model(Unproven(Why::Collections), &[]), ""),
    entry("replaceWith", Model(Unproven(Why::Collections), &[]), ""),
    entry("reserve", Model(Pitted, &[]), ""),
    entry("reserveHint", Model(Unproven(Why::Collections), &[]), ""),
    entry("resize", Model(Unproven(Why::Collections), &[Path("alloc::vec::Vec::resize"), Path("alloc::collections::vec_deque::VecDeque::resize")]), ""),
    entry("resizeWith", Model(Unproven(Why::Collections), &[]), ""),
    entry("retain", Model(Unproven(Why::Collections), &[]), ""),
    entry("retainBack", Model(Unproven(Why::Collections), &[]), ""),
    entry("retainDeque", Model(Unproven(Why::Collections), &[]), ""),
    entry("retainMap", Model(Unproven(Why::Collections), &[]), ""),
    entry("retainMut", Model(Unproven(Why::Collections), &[]), ""),
    entry("retainSet", Model(Unproven(Why::Collections), &[]), ""),
    entry("retainText", Model(Unproven(Why::Collections), &[]), ""),
    entry("rev", Model(Pitted, &[Path("core::iter::Rev")]), ""),
    entry("roomFor", Model(Unproven(Why::Collections), &[Row(StdRow::CapacityOverflow)]), ""),
    entry("roomForCounted", Model(Unproven(Why::Collections), &[]), ""),
    entry("roomForHint", Model(Unproven(Why::Collections), &[Row(StdRow::CapacityOverflow), Row(StdRow::VecFromTrustedLen)]), ""),
    entry("rotateLeft", Model(Unproven(Why::Collections), &[Row(StdRow::RotateLeft), Row(StdRow::VecDequeRotateLeft)]), ""),
    entry("rotateRight", Model(Unproven(Why::Collections), &[Row(StdRow::RotateRight), Row(StdRow::VecDequeRotateRight)]), ""),
    entry("saturatingOp", Form(&[Row(StdRow::SaturatingOp)]), "the lowering's, std_types/number.rs"),
    entry("seekDefaults", Model(Unproven(Why::Outside), &[Row(StdRow::ByteSliceConsume), Row(StdRow::VecDequeDrain)]), "its rows of `$stdSites`, std's sites of its function's panics"),
    entry("selectNthUnstable", Model(Unproven(Why::Sort), &[Row(StdRow::SelectNth)]), ""),
    entry("setReplace", Model(Unproven(Why::Collections), &[]), ""),
    entry("setTake", Model(Unproven(Why::Collections), &[]), ""),
    entry("shift", Model(Unproven(Why::Collections), &[]), ""),
    entry("showBorrowed", Model(Unproven(Why::Format), &[]), ""),
    entry("sink", Model(Unproven(Why::Format), &[]), ""),
    entry("sizeHint", Glue, ""),
    entry("skipping", Model(Pitted, &[Path("core::iter::Skip"), Row(StdRow::SkipCount), Row(StdRow::SkipLast)]), ""),
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
    entry("splitAt", Model(Unproven(Why::Outside), &[Row(StdRow::StrSplitAt)]), "its rows of `$stdSites`, std's sites of its function's panics"),
    entry("splitAtMut", Model(Unproven(Why::Collections), &[]), ""),
    entry("splitChunkMut", Model(Unproven(Why::Collections), &[]), ""),
    entry("splitEndMut", Model(Unproven(Why::Collections), &[]), ""),
    entry("splitOff", Model(Unproven(Why::Collections), &[]), ""),
    entry("stdSites", Glue, ""),
    entry("step", Model(Unproven(Why::Outside), &[Row(StdRow::StepBy)]), "its rows of `$stdSites`, std's sites of its function's panics"),
    entry("stepBy", Model(Pitted, &[Path("core::iter::StepBy")]), ""),
    entry("stepByEnds", Model(Pitted, &[Path("core::iter::StepBy")]), ""),
    entry("stepsOf", Model(Unproven(Why::JsOwn), &[]), "ADR 0570, amended (#2039): a type parameter's value std's compiled body is given, as it is, never armed to close at its end"),
    entry("strDrain", Model(Unproven(Why::Collections), &[]), ""),
    entry("strExtendWithin", Model(Unproven(Why::Collections), &[]), ""),
    entry("strPop", Model(Unproven(Why::Collections), &[]), ""),
    entry("strRemove", Model(Unproven(Why::Collections), &[]), ""),
    entry("strSplitAtMut", Model(Unproven(Why::Outside), &[Row(StdRow::StrSplitAtMut)]), "its rows of `$stdSites`, std's sites of its function's panics"),
    entry("strSplitOff", Model(Unproven(Why::Collections), &[]), ""),
    entry("strTruncate", Model(Unproven(Why::Collections), &[]), ""),
    entry("stringPanic", Glue, ""),
    entry("swap", Model(Unproven(Why::Collections), &[Row(StdRow::VecDequeSwap)]), ""),
    entry("swapRemove", Model(Unproven(Why::Collections), &[Row(StdRow::SwapRemove)]), ""),
    entry("swapWithSlice", Model(Unproven(Why::Collections), &[]), ""),
    entry("tableRoomFor", Model(Unproven(Why::Outside), &[Row(StdRow::HashTableCapacityOverflow)]), "its rows of `$stdSites`, std's sites of its function's panics"),
    entry("take", Model(Unproven(Why::Collections), &[]), ""),
    entry("taking", Model(Pitted, &[Path("core::iter::Take")]), ""),
    entry("testRuns", Glue, ""),
    entry("tests", Glue, ""),
    entry("toVec", Form(&[Path("alloc::slice::<[T]>::to_vec")]), "the lowering's, items.rs and std_types/vec.rs: a map of each item's clone"),
    entry("took", Glue, ""),
    entry("traitImpl", Glue, ""),
    entry("treeAppend", Model(Unproven(Why::Collections), &[]), ""),
    entry("treeEnd", Model(Unproven(Why::Collections), &[]), ""),
    entry("treeFrom", Model(Unproven(Why::Collections), &[]), ""),
    entry("treeFromSorted", Model(Unproven(Why::Collections), &[]), ""),
    entry("treeRange", Model(Unproven(Why::Collections), &[Row(StdRow::BtreeRange)]), ""),
    entry("treeSearchBound", Model(Unproven(Why::Outside), &[Row(StdRow::BtreeRange)]), "its rows of `$stdSites`, std's sites of its function's panics"),
    entry("treeSplitOff", Model(Unproven(Why::Collections), &[]), ""),
    entry("truncate", Model(Unproven(Why::Collections), &[]), ""),
    entry("trustedZip", Model(Pitted, &[Path("core::iter::Zip")]), ""),
    entry("trustedZipEnds", Model(Pitted, &[Path("core::iter::Zip")]), ""),
    entry("tryFoldOk", Model(Unproven(Why::Consumers), &[]), ""),
    entry("tryOf", Model(Pitted, &[]), ""),
    entry("tupleClone", Form(&[Row(StdRow::TupleClone)]), "the lowering's, std_impls/clones.rs"),
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
    entry("vecDequeRange", Form(&[Row(StdRow::VecDequeRange)]), "the lowering's, std_calls.rs"),
    entry("vecDequeSplitOff", Form(&[Row(StdRow::VecDequeSplitOff)]), "the lowering's, combinators.rs"),
    entry("vecDrain", Form(&[Row(StdRow::VecDrain)]), "the lowering's, std_types/text.rs, std_calls.rs"),
    entry("vecExtendFromWithin", Form(&[Row(StdRow::VecExtendFromWithin), Path("alloc::vec::Vec::extend_from_within")]), "the lowering's, std_types/text.rs, std_calls.rs"),
    entry("vecExtractIf", Form(&[Row(StdRow::VecExtractIf)]), "the lowering's, std_types/slice.rs"),
    entry("view", Glue, ""),
    entry("viewGet", Glue, ""),
    entry("viewRange", Glue, ""),
    entry("withCapacity", Model(Unproven(Why::Collections), &[]), ""),
    entry("wrappingOp", Form(&[Row(StdRow::WrappingOp)]), "the lowering's, std_types/number.rs"),
    entry("writeCloneOfSlice", Model(Unproven(Why::Outside), &[Row(StdRow::WriteCloneOfSlice), Path("core::mem::<[MaybeUninit<T>]>::write_clone_of_slice")]), "its rows of `$stdSites`, std's sites of its function's panics"),
    entry("writeCopyOfSlice", Form(&[Row(StdRow::WriteCopyOfSlice)]), "the lowering's, std_types/slice.rs"),
    entry("writeDebugTuple", Model(Unproven(Why::Format), &[Path("core::fmt::Formatter::debug_tuple")]), ""),
    entry("writeInteger", Model(Unproven(Why::Format), &[Path("core::fmt::Formatter::pad_integral")]), "an integer's `Display` or `Debug`"),
    entry("zipEnds", Model(Pitted, &[Path("core::iter::Zip")]), ""),
];

/// The catalog as JSON, which `rust-js --std-models` prints: each helper,
/// whether it's glue, its proof and reason, the std items it stands for,
/// and its note; and each reason's text.
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
                    Row(row) => format!("$stdSites.{}", row.name()),
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
    // A slice's own, `core::slice::<[T]>::fill`, or of a slice of a type,
    // `<[MaybeUninit<T>]>`'s: a method of the impls of a slice in its
    // crate, of that name, the module before it where it's written.
    if parent.last().is_some_and(|last| last.starts_with("<[")) {
        let krate = super::std_crate_num(tcx, krate)?;
        return (tcx
            .incoherent_impls(rustc_middle::ty::fast_reject::SimplifiedType::Slice)
            .iter())
        .filter(|imp| imp.krate == krate)
        .flat_map(|&imp| tcx.associated_item_def_ids(imp).iter().copied())
        .find(|&item| tcx.item_name(item).as_str() == *method);
    }
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

/// Each row of `StdRow`'s no model or form of the catalog says it reads:
/// a row whose reader the catalog doesn't name (ADR 1202, amended).
pub(crate) fn rows_unnamed() -> Vec<&'static str> {
    rows_unnamed_of(MODELS)
}

/// Each row of `StdRow`'s no model or form of `models` says it reads.
fn rows_unnamed_of(models: &[Entry]) -> Vec<&'static str> {
    (StdRow::ALL.iter().copied())
        .filter(|&row| {
            !models.iter().any(|entry| match entry.kind {
                Model(_, stands) | Form(stands) => stands.contains(&Row(row)),
                Glue => false,
            })
        })
        .map(|row| row.name())
        .collect()
}

/// Each std item a model or a form of the catalog stands for, at its path:
/// what `bun run std` walks of the catalog, a clone's site of each that
/// clones what it's given (`compiled_std::cloned_at`, #2577).
pub(crate) fn bound_paths() -> impl Iterator<Item = &'static str> {
    (MODELS.iter())
        .flat_map(|entry| match entry.kind {
            Model(_, stands) | Form(stands) => stands,
            Glue => &[],
        })
        .filter_map(|stands| match *stands {
            Path(path) => Some(path),
            Row(_) => None,
        })
}

/// The path a model or a form of the catalog stands for `item` at, an
/// iterator struct a call makes, `Cloned`'s, its steps' rows' keys of
/// `clonedAt`, `<path>::<step>` (#2577); a std function's own row is keyed
/// by the function a call is resolved to (`call_key`, #2509).
pub(crate) fn path_of(tcx: TyCtxt<'_>, item: DefId) -> Option<&'static str> {
    let name = tcx.opt_item_name(item)?;
    bound_paths().find(|path| path.rsplit("::").next() == Some(name.as_str()) && resolved(tcx, path) == Some(item))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Each row of `$stdSites` is named by the catalog, a model's or a
    /// form's, so a row is never read by a reader it doesn't name; and of a
    /// catalog without a row's reader, the row is said.
    #[test]
    fn each_row_is_the_catalogs() {
        assert_eq!(rows_unnamed(), Vec::<&str>::new());
        let lacking: Vec<Entry> = (MODELS.iter().copied())
            .filter(|entry| entry.helper != "vecDrain")
            .collect();
        assert_eq!(rows_unnamed_of(&lacking), vec!["vecDrain"]);
    }
}
