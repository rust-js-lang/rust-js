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

use rustc_hir::def::DefKind;
use rustc_middle::ty::TyCtxt;
use rustc_span::def_id::DefId;

/// Why no case pits a model against std's body: std's build of its
/// iterator structs has no body of its own in its place.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Why {
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

/// What a helper is: glue, or a model of the std items it stands for,
/// none yet where it isn't bound.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Kind {
    Glue,
    Model(Proof, &'static [&'static str]),
}

/// One of the runtime's helpers, by its name, the export `$` and it, what
/// it is, and a note, its ADR.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Entry {
    pub helper: &'static str,
    pub kind: Kind,
    pub note: &'static str,
}

use Kind::{Glue, Model};
use Proof::{Pitted, Unproven};

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
    entry("add", Model(Unproven(Why::Collections), &[]), ""),
    entry("advanceSlice", Model(Unproven(Why::JsOwn), &[]), "ADRs 0422, 0465: an array's iterator an adapter holds, advanced by index"),
    entry("andModify", Model(Unproven(Why::Collections), &[]), ""),
    entry("append", Model(Unproven(Why::Collections), &[]), ""),
    entry("asChunksMut", Model(Unproven(Why::Collections), &[]), ""),
    entry("asciiCase", Model(Unproven(Why::Collections), &[]), ""),
    entry("assign", Glue, ""),
    entry("at", Glue, ""),
    entry("binarySearchBy", Model(Unproven(Why::Collections), &[]), ""),
    entry("borrow", Model(Unproven(Why::Cells), &["core::cell::RefCell::borrow"]), ""),
    entry("borrowMut", Model(Unproven(Why::Cells), &["core::cell::RefCell::borrow_mut"]), ""),
    entry("btreeSet", Model(Unproven(Why::Collections), &[]), ""),
    entry("byteLen", Model(Unproven(Why::Collections), &[]), ""),
    entry("capacity", Model(Unproven(Why::Collections), &[]), ""),
    entry("catchUnwind", Glue, ""),
    entry("catchable", Glue, ""),
    entry("chars", Model(Unproven(Why::JsOwn), &["core::str::Chars"]), "ADR 1117: a string's `Chars`, the runtime's lazy one"),
    entry("checkedSplitAtMut", Model(Unproven(Why::Collections), &[]), ""),
    entry("chunksExactMut", Model(Unproven(Why::Collections), &[]), ""),
    entry("chunksMut", Model(Unproven(Why::Collections), &[]), ""),
    entry("clampBy", Model(Unproven(Why::Collections), &[]), ""),
    entry("cleanup", Glue, ""),
    entry("clone", Model(Pitted, &[]), ""),
    entry("cloneFromSlice", Model(Unproven(Why::Collections), &[]), ""),
    entry("cloneIntoVec", Model(Unproven(Why::Collections), &[]), ""),
    entry("closing", Glue, ""),
    entry("cmp", Glue, ""),
    entry("cmpItems", Model(Unproven(Why::Consumers), &[]), ""),
    entry("collectDropping", Model(Pitted, &["core::iter::Iterator::collect"]), ""),
    entry("collectOptions", Model(Unproven(Why::Consumers), &[]), ""),
    entry("collectResults", Model(Unproven(Why::Consumers), &[]), ""),
    entry("collectVec", Model(Unproven(Why::Consumers), &[]), ""),
    entry("collectsInPlace", Model(Unproven(Why::Consumers), &[]), ""),
    entry("consume", Model(Pitted, &[]), ""),
    entry("copyFromSlice", Model(Unproven(Why::Collections), &[]), ""),
    entry("copyRange", Model(Unproven(Why::Collections), &[]), ""),
    entry("copyWithin", Model(Unproven(Why::Collections), &[]), ""),
    entry("count", Model(Unproven(Why::Consumers), &[]), ""),
    entry("cycleOf", Model(Pitted, &["core::iter::Cycle"]), ""),
    entry("debugDuration", Model(Unproven(Why::Format), &[]), ""),
    entry("debugF64", Model(Unproven(Why::Format), &[]), ""),
    entry("debugList", Model(Unproven(Why::Format), &[]), ""),
    entry("debugMap", Model(Unproven(Why::Format), &[]), ""),
    entry("debugParseError", Model(Unproven(Why::Format), &[]), ""),
    entry("debugSet", Model(Unproven(Why::Format), &[]), ""),
    entry("debugStr", Model(Unproven(Why::Format), &[]), ""),
    entry("decodeUtf16", Model(Unproven(Why::Iterators), &["core::char::DecodeUtf16"]), "ADR 0327, #1584: `char::decode_utf16`'s, whose error is its code, where std's body makes `DecodeUtf16Error { code }`"),
    entry("dedupBy", Model(Unproven(Why::Collections), &[]), ""),
    entry("dedupByCells", Model(Unproven(Why::Collections), &[]), ""),
    entry("dequeInsert", Model(Unproven(Why::Collections), &[]), ""),
    entry("dequeSwapRemove", Model(Unproven(Why::Collections), &[]), ""),
    entry("dispose", Glue, ""),
    entry("disposeIterator", Model(Unproven(Why::JsOwn), &[]), "ADR 0140, ADR 1136: a `dyn Iterator`'s JS iterator, a binding's, closed as JS's own"),
    entry("drain", Model(Unproven(Why::Collections), &[]), ""),
    entry("drainAll", Model(Unproven(Why::Collections), &[]), ""),
    entry("dropEach", Glue, ""),
    entry("dropItems", Glue, ""),
    entry("dropping", Glue, ""),
    entry("droppingIterator", Model(Unproven(Why::JsOwn), &[]), "ADR 0140, ADR 1136: a `dyn Iterator`'s JS iterator stepped dropping what it gives, as JS's own"),
    entry("endEntry", Model(Unproven(Why::Collections), &[]), ""),
    entry("entry", Model(Unproven(Why::Collections), &[]), ""),
    entry("entryGet", Model(Unproven(Why::Collections), &[]), ""),
    entry("entryInsert", Model(Unproven(Why::Collections), &[]), ""),
    entry("entryOrInsert", Model(Unproven(Why::Collections), &[]), ""),
    entry("entryOrInsertWith", Model(Unproven(Why::Collections), &[]), ""),
    entry("entryRemove", Model(Unproven(Why::Collections), &[]), ""),
    entry("enumerate", Model(Pitted, &["core::iter::Enumerate"]), ""),
    entry("eq", Glue, ""),
    entry("exactLen", Model(Pitted, &["core::iter::ExactSizeIterator::len"]), ""),
    entry("exchange", Glue, ""),
    entry("extend", Model(Unproven(Why::Collections), &[]), ""),
    entry("extendMap", Model(Unproven(Why::Collections), &[]), ""),
    entry("extendText", Model(Unproven(Why::Collections), &[]), ""),
    entry("extractIf", Model(Unproven(Why::Collections), &[]), ""),
    entry("fill", Model(Unproven(Why::Collections), &[]), ""),
    entry("fillWith", Model(Unproven(Why::Collections), &[]), ""),
    entry("filterDropping", Model(Pitted, &["core::iter::Filter"]), ""),
    entry("filterEnds", Model(Pitted, &["core::iter::Filter"]), ""),
    entry("findBack", Model(Pitted, &["core::iter::DoubleEndedIterator::rfind"]), ""),
    entry("findDropping", Model(Unproven(Why::Consumers), &[]), ""),
    entry("flatMap", Model(Pitted, &["core::iter::FlatMap"]), ""),
    entry("flattenEnds", Model(Pitted, &["core::iter::Flatten"]), ""),
    entry("foldBack", Model(Pitted, &["core::iter::DoubleEndedIterator::rfold"]), ""),
    entry("foldDropping", Model(Unproven(Why::Consumers), &[]), ""),
    entry("foldOf", Model(Unproven(Why::Consumers), &[]), ""),
    entry("foldToArray", Model(Unproven(Why::Consumers), &[]), ""),
    entry("folded", Model(Unproven(Why::Consumers), &[]), ""),
    entry("formatting", Model(Unproven(Why::Format), &[]), ""),
    entry("getDisjointMut", Model(Unproven(Why::Collections), &[]), ""),
    entry("given", Glue, ""),
    entry("givenDrop", Glue, ""),
    entry("grew", Model(Unproven(Why::Collections), &[]), ""),
    entry("grownBy", Model(Unproven(Why::Collections), &[]), ""),
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
    entry("iter", Model(Unproven(Why::JsOwn), &["core::slice::Iter"]), "ADR 0071: an array's iterator, which knows where it is"),
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
    entry("lazyChain", Model(Pitted, &["core::iter::Chain"]), ""),
    entry("lazyChunkBy", Model(Unproven(Why::Iterators), &["core::slice::ChunkBy"]), ""),
    entry("lazyChunkByMut", Model(Unproven(Why::Iterators), &["core::slice::ChunkByMut"]), ""),
    entry("lazyMapWhile", Model(Pitted, &["core::iter::MapWhile"]), ""),
    entry("lazyRange", Model(Pitted, &["core::ops::Range"]), ""),
    entry("lazyScan", Model(Pitted, &["core::iter::Scan"]), ""),
    entry("lazySkipWhile", Model(Pitted, &["core::iter::SkipWhile"]), ""),
    entry("lazySliceSplitBy", Model(Unproven(Why::Iterators), &[]), ""),
    entry("lazySliceSplitByMut", Model(Unproven(Why::Iterators), &[]), ""),
    entry("lazyTakeWhile", Model(Pitted, &["core::iter::TakeWhile"]), ""),
    entry("lazyZip", Model(Pitted, &["core::iter::Zip"]), ""),
    entry("lent", Model(Pitted, &[]), ""),
    entry("live", Glue, ""),
    entry("madeEach", Glue, ""),
    entry("madeInTurn", Glue, ""),
    entry("main", Glue, "ADR 1198: a binary's start, which every case's binary imports"),
    entry("makeMut", Glue, ""),
    entry("mapEnds", Model(Pitted, &["core::iter::Map"]), ""),
    entry("mapExtractIf", Model(Unproven(Why::Collections), &[]), ""),
    entry("mapFrom", Model(Unproven(Why::Collections), &[]), ""),
    entry("mapGetDisjointMut", Model(Unproven(Why::Collections), &[]), ""),
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
    entry("nextBack", Model(Pitted, &["core::iter::DoubleEndedIterator::next_back"]), ""),
    entry("nextBackSome", Model(Unproven(Why::JsOwn), &[]), "ADRs 0164, 0051: the step from its back of any JS iterator, its item boxed where it may look like `None`, of an `Option`'s items (#1826)"),
    entry("nextSome", Model(Unproven(Why::JsOwn), &[]), "ADRs 0140, 0051: the step of any JS iterator, its item boxed where it may look like `None`"),
    entry("nth", Model(Unproven(Why::Consumers), &[]), ""),
    entry("nthBack", Model(Unproven(Why::Consumers), &[]), ""),
    entry("nthDropping", Model(Unproven(Why::Consumers), &[]), ""),
    entry("orInsert", Model(Unproven(Why::Collections), &[]), ""),
    entry("orInsertWith", Model(Unproven(Why::Collections), &[]), ""),
    entry("orInsertWithKey", Model(Unproven(Why::Collections), &[]), ""),
    entry("ord", Glue, ""),
    entry("ordTreeMap", Model(Unproven(Why::Collections), &[]), ""),
    entry("ordTreeSet", Model(Unproven(Why::Collections), &[]), ""),
    entry("out", Model(Unproven(Why::Format), &[]), ""),
    entry("panic", Glue, ""),
    entry("parseInt", Model(Unproven(Why::Numbers), &[]), ""),
    entry("partialEq", Glue, ""),
    entry("partialOrd", Glue, ""),
    entry("partiallyOrdered", Glue, ""),
    entry("partition", Model(Unproven(Why::Consumers), &[]), ""),
    entry("partitionPoint", Model(Unproven(Why::Collections), &[]), ""),
    entry("pathJoin", Model(Unproven(Why::Collections), &[]), ""),
    entry("peek", Model(Unproven(Why::Memory), &["core::iter::Peekable::peek"]), "ADR 0561: `Peekable::peek`, whose body reaches memory"),
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
    entry("range", Model(Pitted, &["core::ops::Range"]), ""),
    entry("rangeCountOut", Model(Unproven(Why::Ranges), &[]), ""),
    entry("rangeFirst", Model(Unproven(Why::Ranges), &[]), ""),
    entry("rangeLast", Model(Unproven(Why::Ranges), &[]), ""),
    entry("rangeLastOut", Model(Unproven(Why::Ranges), &[]), ""),
    entry("rangeLen", Model(Pitted, &["core::ops::Range"]), ""),
    entry("rangeNext", Model(Unproven(Why::Ranges), &["core::ops::Range"]), ""),
    entry("rangeNextBack", Model(Pitted, &["core::ops::Range"]), ""),
    entry("rangeNth", Model(Unproven(Why::Ranges), &["core::ops::Range"]), ""),
    entry("rangeNthBack", Model(Unproven(Why::Ranges), &["core::ops::Range"]), ""),
    entry("rcClone", Glue, ""),
    entry("rcDrop", Glue, ""),
    entry("rchunksMut", Model(Unproven(Why::Collections), &[]), ""),
    entry("reduce", Model(Unproven(Why::Consumers), &[]), ""),
    entry("remove", Model(Unproven(Why::Collections), &[]), ""),
    entry("removeEntry", Model(Unproven(Why::Collections), &[]), ""),
    entry("removeItem", Model(Unproven(Why::Collections), &[]), ""),
    entry("removeOpt", Model(Unproven(Why::Collections), &[]), ""),
    entry("removeOutOfBounds", Model(Unproven(Why::Collections), &[]), ""),
    entry("repeat", Model(Unproven(Why::Collections), &[]), "`vec![item; count]`, `Vec::from_elem`"),
    entry("repeating", Model(Pitted, &["core::iter::Repeat"]), ""),
    entry("repeatingN", Model(Pitted, &["core::iter::RepeatN"]), ""),
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
    entry("rev", Model(Pitted, &["core::iter::Rev"]), ""),
    entry("roomFor", Model(Unproven(Why::Collections), &[]), ""),
    entry("roomForCounted", Model(Unproven(Why::Collections), &[]), ""),
    entry("roomForHint", Model(Unproven(Why::Collections), &[]), ""),
    entry("rotateLeft", Model(Unproven(Why::Collections), &[]), ""),
    entry("rotateRight", Model(Unproven(Why::Collections), &[]), ""),
    entry("selectNthUnstable", Model(Unproven(Why::Sort), &[]), ""),
    entry("setReplace", Model(Unproven(Why::Collections), &[]), ""),
    entry("setTake", Model(Unproven(Why::Collections), &[]), ""),
    entry("shift", Model(Unproven(Why::Collections), &[]), ""),
    entry("showBorrowed", Model(Unproven(Why::Format), &[]), ""),
    entry("sink", Model(Unproven(Why::Format), &[]), ""),
    entry("sizeHint", Glue, ""),
    entry("skipping", Model(Pitted, &["core::iter::Skip"]), ""),
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
    entry("splitAtMut", Model(Unproven(Why::Collections), &[]), ""),
    entry("splitChunkMut", Model(Unproven(Why::Collections), &[]), ""),
    entry("splitEndMut", Model(Unproven(Why::Collections), &[]), ""),
    entry("splitOff", Model(Unproven(Why::Collections), &[]), ""),
    entry("stdSites", Glue, ""),
    entry("stepBy", Model(Pitted, &["core::iter::StepBy"]), ""),
    entry("stepByEnds", Model(Pitted, &["core::iter::StepBy"]), ""),
    entry("stepsOf", Model(Unproven(Why::JsOwn), &[]), "ADR 0570, amended (#2039): a type parameter's value std's compiled body is given, as it is, never armed to close at its end"),
    entry("strDrain", Model(Unproven(Why::Collections), &[]), ""),
    entry("strExtendWithin", Model(Unproven(Why::Collections), &[]), ""),
    entry("strPop", Model(Unproven(Why::Collections), &[]), ""),
    entry("strRemove", Model(Unproven(Why::Collections), &[]), ""),
    entry("strSplitOff", Model(Unproven(Why::Collections), &[]), ""),
    entry("strTruncate", Model(Unproven(Why::Collections), &[]), ""),
    entry("stringPanic", Glue, ""),
    entry("swap", Model(Unproven(Why::Collections), &[]), ""),
    entry("swapRemove", Model(Unproven(Why::Collections), &[]), ""),
    entry("swapWithSlice", Model(Unproven(Why::Collections), &[]), ""),
    entry("take", Model(Unproven(Why::Collections), &[]), ""),
    entry("taking", Model(Pitted, &["core::iter::Take"]), ""),
    entry("took", Glue, ""),
    entry("traitImpl", Glue, ""),
    entry("treeAppend", Model(Unproven(Why::Collections), &[]), ""),
    entry("treeEnd", Model(Unproven(Why::Collections), &[]), ""),
    entry("treeFrom", Model(Unproven(Why::Collections), &[]), ""),
    entry("treeFromSorted", Model(Unproven(Why::Collections), &[]), ""),
    entry("treeRange", Model(Unproven(Why::Collections), &[]), ""),
    entry("treeSplitOff", Model(Unproven(Why::Collections), &[]), ""),
    entry("truncate", Model(Unproven(Why::Collections), &[]), ""),
    entry("trustedZip", Model(Pitted, &["core::iter::Zip"]), ""),
    entry("trustedZipEnds", Model(Pitted, &["core::iter::Zip"]), ""),
    entry("tryFoldOk", Model(Unproven(Why::Consumers), &[]), ""),
    entry("tryOf", Model(Pitted, &[]), ""),
    entry("unborrow", Model(Unproven(Why::Cells), &[]), ""),
    entry("unfused", Model(Pitted, &["core::iter::Fuse"]), ""),
    entry("unfusedUnless", Model(Pitted, &["core::iter::Fuse"]), ""),
    entry("untracked", Model(Pitted, &[]), ""),
    entry("unwinds", Glue, ""),
    entry("unwindsOn", Glue, ""),
    entry("unwound", Glue, ""),
    entry("unwrap", Glue, ""),
    entry("unwrapOk", Glue, ""),
    entry("unzip", Model(Unproven(Why::Consumers), &[]), ""),
    entry("upgrade", Glue, ""),
    entry("view", Glue, ""),
    entry("viewGet", Glue, ""),
    entry("viewRange", Glue, ""),
    entry("withCapacity", Model(Unproven(Why::Collections), &[]), ""),
    entry("zipEnds", Model(Pitted, &["core::iter::Zip"]), ""),
];

/// The catalog as JSON, which `rust-js --std-models` prints: each helper,
/// whether it's glue, its proof and reason, the std items it stands for,
/// and its note; and each reason's text.
pub fn json() -> String {
    let entries: Vec<serde_json::Value> = MODELS
        .iter()
        .map(|entry| {
            let (kind, pitted, why, stands_for): (&str, bool, Option<&str>, &[&str]) = match entry.kind {
                Glue => ("glue", false, Some("glue"), &[]),
                Model(Pitted, paths) => ("model", true, None, paths),
                Model(Unproven(why), paths) => ("model", false, Some(why.name()), paths),
            };
            serde_json::json!({
                "helper": format!("${}", entry.helper),
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
        let Model(_, paths) = entry.kind else { continue };
        for path in paths {
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
