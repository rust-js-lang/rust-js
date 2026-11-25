//! The rows of `$stdSites` whose sites `bun run std` reads from std's MIR,
//! each by its name and the std function whose panics it holds: the
//! catalog's, which each model's `Row` names (ADR 1202, amended; Campaign
//! 5's step 2). `compiled_std::raised` walks them. Split from models.rs by
//! concept, as a file is at most 1350 lines.

use crate::recognition::StdCrate;
use rustc_hir::attrs::lang_items::LangItem;
use rustc_middle::ty::fast_reject::SimplifiedType;
use rustc_middle::ty::{FloatTy, IntTy, Ty, TyCtxt, UintTy};

/// Each row of `$stdSites`, by its name there, each variant's one handle,
/// and `ALL` of them: of a row the lowering reads, its reader names it
/// through `StdRow`'s methods, never by its text, so the catalog finds its
/// readers by the API (Campaign 5's step 2c, ADR 1202 amended).
macro_rules! std_rows {
    ($($row:ident = $name:literal,)*) => {
        #[derive(Clone, Copy, PartialEq, Eq, Debug)]
        pub enum StdRow {
            $($row,)*
        }

        impl StdRow {
            /// Every row, in the order they're listed.
            pub const ALL: &'static [StdRow] = &[$(StdRow::$row,)*];

            /// Its name in `$stdSites`.
            pub fn name(self) -> &'static str {
                match self {
                    $(StdRow::$row => $name,)*
                }
            }
        }
    };
}

#[rustfmt::skip]
std_rows! {
    ArcFromTrustedLen = "arcFromTrustedLen",
    ArcSliceLayout = "arcSliceLayout",
    BoxLast = "boxLast",
    BtreeRange = "btreeRange",
    BufReaderStreamPosition = "bufReaderStreamPosition",
    ByteSliceConsume = "byteSliceConsume",
    CapacityOverflow = "capacityOverflow",
    ChainCount = "chainCount",
    Clamp = "clamp",
    ClonedAt = "clonedAt",
    CopiedCount = "copiedCount",
    CopiedLast = "copiedLast",
    DurationAdd = "durationAdd",
    DurationNew = "durationNew",
    DurationSub = "durationSub",
    EncodeUtf16 = "encodeUtf16",
    EncodeUtf8 = "encodeUtf8",
    EnumerateCount = "enumerateCount",
    ExactSizeLen = "exactSizeLen",
    FlattenCount = "flattenCount",
    FlattenLast = "flattenLast",
    FnOnceCall = "fnOnceCall",
    Format = "format",
    FromDigit = "fromDigit",
    FromStrRadix = "fromStrRadix",
    HashTableCapacityOverflow = "hashTableCapacityOverflow",
    LazyCellPoisoned = "lazyCellPoisoned",
    LazyLockPoisoned = "lazyLockPoisoned",
    LocalKeyReplace = "localKeyReplace",
    LocalKeySet = "localKeySet",
    LocalKeyWithBorrow = "localKeyWithBorrow",
    LocalKeyWithBorrowMut = "localKeyWithBorrowMut",
    MapGetDisjointMut = "mapGetDisjointMut",
    NextMultipleOf = "nextMultipleOf",
    OnceCellReentrant = "onceCellReentrant",
    OrdClamp = "ordClamp",
    PanicNounwind = "panicNounwind",
    PanicNounwindNobacktrace = "panicNounwindNobacktrace",
    PathExtension = "pathExtension",
    PrintFailed = "printFailed",
    RcFromTrustedLen = "rcFromTrustedLen",
    RcSliceLayout = "rcSliceLayout",
    RefCellEq = "refCellEq",
    RefCellSwap = "refCellSwap",
    RefCellTake = "refCellTake",
    RotateLeft = "rotateLeft",
    RotateRight = "rotateRight",
    SaturatingOp = "saturatingOp",
    SelectNth = "selectNth",
    SkipCount = "skipCount",
    SkipLast = "skipLast",
    SliceRepeat = "sliceRepeat",
    SortTotalOrder = "sortTotalOrder",
    StepBy = "stepBy",
    StepForward = "stepForward",
    StrSplitAt = "strSplitAt",
    StrSplitAtMut = "strSplitAtMut",
    SwapRemove = "swapRemove",
    ToString = "toString",
    TupleClone = "tupleClone",
    VecDequeDrain = "vecDequeDrain",
    VecDequeExtendTrusted = "vecDequeExtendTrusted",
    VecDequeInsert = "vecDequeInsert",
    VecDequeRange = "vecDequeRange",
    VecDequeReserve = "vecDequeReserve",
    VecDequeRotateLeft = "vecDequeRotateLeft",
    VecDequeRotateRight = "vecDequeRotateRight",
    VecDequeSplitOff = "vecDequeSplitOff",
    VecDequeSwap = "vecDequeSwap",
    VecDrain = "vecDrain",
    VecExtendFromWithin = "vecExtendFromWithin",
    VecExtendTrusted = "vecExtendTrusted",
    VecExtractIf = "vecExtractIf",
    VecFromTrustedLen = "vecFromTrustedLen",
    WrappingOp = "wrappingOp",
    WriteCloneOfSlice = "writeCloneOfSlice",
    WriteCopyOfSlice = "writeCopyOfSlice",
}

impl StdRow {
    /// The row of `name`, as `bun run std` writes it.
    pub fn named(name: &str) -> Option<StdRow> {
        StdRow::ALL.iter().copied().find(|row| row.name() == name)
    }
}

/// A trait: by its lang item, as a derive macro has its name, or at its
/// path, of one that isn't one.
#[derive(Clone, Copy, Debug)]
pub(crate) enum Trait {
    Lang(LangItem),
    Path(StdCrate, &'static [&'static str]),
}

/// Whose a function is: a type's own, at its path in its crate; a
/// primitive's own; a trait's impl, the trait by its lang item, as a
/// derive macro has its name, for a type at its path; or a free function,
/// at its path, its name the path's last.
#[derive(Clone, Copy, Debug)]
pub(crate) enum Owner {
    Type(StdCrate, &'static [&'static str]),
    /// A type's own impl of the type over another, the second path,
    /// a thread-local of a `RefCell`, apart from one of a `Cell`.
    TypeOver(StdCrate, &'static [&'static str], (StdCrate, &'static [&'static str])),
    Primitive(SimplifiedType),
    /// A trait's impl, the first path, for a type, the second, or for any
    /// type, `None`, of those std specializes, the one bound by a trait,
    /// the third: `SpecFromIterNested`'s for a `TrustedLen`.
    Bounded(
        (StdCrate, &'static [&'static str]),
        Option<(StdCrate, &'static [&'static str])>,
        (StdCrate, &'static [&'static str]),
    ),
    Impl {
        trait_: Trait,
        of: (StdCrate, &'static [&'static str]),
    },
    Free(StdCrate, &'static [&'static str]),
    /// A trait's own default of a method, the trait at its path.
    Default(StdCrate, &'static [&'static str]),
    /// A trait's declared method, the trait at its path, of which rustc's
    /// shim is made, `Clone::clone`'s of a tuple, `FnMut::call_mut`'s of a
    /// function given as a closure: its site is where it's declared, as
    /// rustc places a shim's calls (#2471).
    Declared(StdCrate, &'static [&'static str]),
    /// A trait's impl for any type, `impl<T: Display> SpecToString for T`,
    /// the trait at its path.
    Blanket(StdCrate, &'static [&'static str]),
    /// A trait's impl for a reference to a slice of bytes, `&[u8]`'s
    /// `BufRead`, the trait at its path.
    ForByteSlice(StdCrate, &'static [&'static str]),
    /// A type's own of a crate std's build vendors, at its path there, the
    /// crate the one a std type's field, the second, is of: hashbrown's
    /// `RawTable`, through std `HashMap`'s `base` (#2325).
    Vendored(
        &'static [&'static str],
        (StdCrate, &'static [&'static str], &'static str),
    ),
    /// A trait's method, the trait at its path, as rustc resolves it for
    /// the type the function makes, of a `Repeat`: of the impls std
    /// specializes, the one that applies to it, `Box<Repeat<u8>>`'s sized
    /// `BoxIter`, a flatten of `Repeat`s' general one, which isn't a
    /// `OneShot`'s (#2488).
    Resolved(
        (StdCrate, &'static [&'static str]),
        for<'tcx> fn(TyCtxt<'tcx>) -> Option<Ty<'tcx>>,
    ),
}

/// Each site of std's panics a helper raises, by its name in `$stdSites`,
/// and a type's where a helper serves several (`u8`), or a kind of type's
/// whose sites are one (`signed`): whose function holds
/// it, and its name, of a free function none, its path's last. A
/// function's sites are in the order they're written.
pub(crate) const ROWS: &[(StdRow, Option<&str>, Owner, &str)] = &[
    (
        StdRow::VecDequeSwap,
        None,
        Owner::Type(StdCrate::Alloc, &["collections", "vec_deque", "VecDeque"]),
        "swap",
    ),
    (
        StdRow::VecDequeInsert,
        None,
        Owner::Type(StdCrate::Alloc, &["collections", "vec_deque", "VecDeque"]),
        "insert_mut",
    ),
    (
        StdRow::VecDequeRotateLeft,
        None,
        Owner::Type(StdCrate::Alloc, &["collections", "vec_deque", "VecDeque"]),
        "rotate_left",
    ),
    (
        StdRow::VecDequeRotateRight,
        None,
        Owner::Type(StdCrate::Alloc, &["collections", "vec_deque", "VecDeque"]),
        "rotate_right",
    ),
    (
        StdRow::SwapRemove,
        None,
        Owner::Type(StdCrate::Alloc, &["vec", "Vec"]),
        "swap_remove",
    ),
    (
        StdRow::CapacityOverflow,
        None,
        Owner::Free(StdCrate::Alloc, &["raw_vec", "capacity_overflow"]),
        "",
    ),
    // A panic that can't unwind, which aborts: std's hook names the line in
    // `core::panicking` its message is raised at, of a function that can't
    // unwind's, `panic_nounwind`'s, and of a drop's panic as another
    // unwinds, `panic_nounwind_nobacktrace`'s (`$abort`, #2452).
    (
        StdRow::PanicNounwind,
        None,
        Owner::Free(StdCrate::Core, &["panicking", "panic_nounwind"]),
        "",
    ),
    (
        StdRow::PanicNounwindNobacktrace,
        None,
        Owner::Free(StdCrate::Core, &["panicking", "panic_nounwind_nobacktrace"]),
        "",
    ),
    (
        StdRow::StrSplitAt,
        None,
        Owner::Primitive(SimplifiedType::Str),
        "split_at",
    ),
    // A `HashMap`'s `get_disjoint_mut` of two keys of one entry: hashbrown's
    // table's, vendored, named under its own build's registry (#2325).
    (
        StdRow::MapGetDisjointMut,
        None,
        Owner::Vendored(
            &["raw", "RawTable"],
            (StdCrate::Std, &["collections", "hash", "map", "HashMap"], "base"),
        ),
        "get_disjoint_mut",
    ),
    (
        StdRow::StrSplitAtMut,
        None,
        Owner::Primitive(SimplifiedType::Str),
        "split_at_mut",
    ),
    // A hash table's room past what its buckets can count: hashbrown's
    // `Fallibility::capacity_overflow`, its `panic!` (#2487).
    (
        StdRow::HashTableCapacityOverflow,
        None,
        Owner::Vendored(
            &["raw", "Fallibility"],
            (StdCrate::Std, &["collections", "hash", "map", "HashMap"], "base"),
        ),
        "capacity_overflow",
    ),
    (
        StdRow::RotateLeft,
        None,
        Owner::Primitive(SimplifiedType::Slice),
        "rotate_left",
    ),
    (
        StdRow::RotateRight,
        None,
        Owner::Primitive(SimplifiedType::Slice),
        "rotate_right",
    ),
    (
        StdRow::SelectNth,
        None,
        Owner::Free(StdCrate::Core, &["slice", "sort", "select", "partition_at_index"]),
        "",
    ),
    (
        StdRow::Clamp,
        Some("f64"),
        Owner::Primitive(SimplifiedType::Float(FloatTy::F64)),
        "clamp",
    ),
    (
        StdRow::Clamp,
        Some("f32"),
        Owner::Primitive(SimplifiedType::Float(FloatTy::F32)),
        "clamp",
    ),
    (
        StdRow::FromDigit,
        None,
        Owner::Free(StdCrate::Core, &["char", "convert", "from_digit"]),
        "",
    ),
    (
        StdRow::EncodeUtf8,
        None,
        Owner::Free(StdCrate::Core, &["char", "encode_utf8_raw"]),
        "",
    ),
    (
        StdRow::EncodeUtf16,
        None,
        Owner::Free(StdCrate::Core, &["char", "encode_utf16_raw"]),
        "",
    ),
    (
        StdRow::StepBy,
        None,
        Owner::Type(StdCrate::Core, &["iter", "StepBy"]),
        "new",
    ),
    (
        StdRow::DurationNew,
        None,
        Owner::Type(StdCrate::Core, &["time", "Duration"]),
        "new",
    ),
    (
        StdRow::DurationAdd,
        None,
        Owner::Impl {
            trait_: Trait::Lang(LangItem::Add),
            of: (StdCrate::Core, &["time", "Duration"]),
        },
        "add",
    ),
    (
        StdRow::DurationSub,
        None,
        Owner::Impl {
            trait_: Trait::Lang(LangItem::Sub),
            of: (StdCrate::Core, &["time", "Duration"]),
        },
        "sub",
    ),
    (
        StdRow::OnceCellReentrant,
        None,
        Owner::Type(StdCrate::Core, &["cell", "OnceCell"]),
        "try_init",
    ),
    (
        StdRow::LazyCellPoisoned,
        None,
        Owner::Free(StdCrate::Core, &["cell", "lazy", "panic_poisoned"]),
        "",
    ),
    (
        StdRow::BtreeRange,
        None,
        Owner::Type(StdCrate::Alloc, &["collections", "btree", "node", "NodeRef"]),
        "search_tree_for_bifurcation",
    ),
    // `RefCell::take`'s borrow, its `replace`'s call, and `swap`'s two,
    // `self`'s and `other`'s.
    (
        StdRow::RefCellTake,
        None,
        Owner::Type(StdCrate::Core, &["cell", "RefCell"]),
        "take",
    ),
    (
        StdRow::RefCellSwap,
        None,
        Owner::Type(StdCrate::Core, &["cell", "RefCell"]),
        "swap",
    ),
    // A thread-local `RefCell`'s: each its `with`'s call, then its borrow.
    (
        StdRow::LocalKeyWithBorrow,
        None,
        Owner::TypeOver(
            StdCrate::Std,
            &["thread", "local", "LocalKey"],
            (StdCrate::Core, &["cell", "RefCell"]),
        ),
        "with_borrow",
    ),
    (
        StdRow::LocalKeyWithBorrowMut,
        None,
        Owner::TypeOver(
            StdCrate::Std,
            &["thread", "local", "LocalKey"],
            (StdCrate::Core, &["cell", "RefCell"]),
        ),
        "with_borrow_mut",
    ),
    (
        StdRow::LocalKeySet,
        None,
        Owner::TypeOver(
            StdCrate::Std,
            &["thread", "local", "LocalKey"],
            (StdCrate::Core, &["cell", "RefCell"]),
        ),
        "set",
    ),
    (
        StdRow::LocalKeyReplace,
        None,
        Owner::TypeOver(
            StdCrate::Std,
            &["thread", "local", "LocalKey"],
            (StdCrate::Core, &["cell", "RefCell"]),
        ),
        "replace",
    ),
    // Where std's own `capacity overflow` panics are, past `raw_vec`'s: a
    // `TrustedLen` of no upper bound, collected into a `Vec`, an `Rc<[T]>`
    // or an `Arc<[T]>`, or extending a `Vec` or a `VecDeque`; and a
    // `VecDeque`'s reserve past `usize::MAX`.
    (
        StdRow::VecFromTrustedLen,
        None,
        Owner::Bounded(
            (StdCrate::Alloc, &["vec", "spec_from_iter_nested", "SpecFromIterNested"]),
            Some((StdCrate::Alloc, &["vec", "Vec"])),
            (StdCrate::Core, &["iter", "traits", "marker", "TrustedLen"]),
        ),
        "from_iter",
    ),
    (
        StdRow::RcFromTrustedLen,
        None,
        Owner::Bounded(
            (StdCrate::Alloc, &["rc", "ToRcSlice"]),
            None,
            (StdCrate::Core, &["iter", "traits", "marker", "TrustedLen"]),
        ),
        "to_rc_slice",
    ),
    (
        StdRow::ArcFromTrustedLen,
        None,
        Owner::Bounded(
            (StdCrate::Alloc, &["sync", "ToArcSlice"]),
            None,
            (StdCrate::Core, &["iter", "traits", "marker", "TrustedLen"]),
        ),
        "to_arc_slice",
    ),
    // A slice's `repeat`, its `checked_mul(n).expect("capacity overflow")`
    // of a count past `usize::MAX` (#2488).
    (
        StdRow::SliceRepeat,
        None,
        Owner::Primitive(SimplifiedType::Slice),
        "repeat",
    ),
    // Where `Rc::<[T]>`'s and `Arc::<[T]>`'s slice is allocated, its
    // `Layout::array::<T>(len).unwrap()`, which a collect past what a
    // `Layout` holds panics at (#2489).
    (
        StdRow::RcSliceLayout,
        None,
        Owner::Type(StdCrate::Alloc, &["rc", "Rc"]),
        "allocate_for_slice",
    ),
    (
        StdRow::ArcSliceLayout,
        None,
        Owner::Type(StdCrate::Alloc, &["sync", "Arc"]),
        "allocate_for_slice",
    ),
    (
        StdRow::VecExtendTrusted,
        None,
        Owner::Type(StdCrate::Alloc, &["vec", "Vec"]),
        "extend_trusted",
    ),
    (
        StdRow::VecDequeExtendTrusted,
        None,
        Owner::Bounded(
            (
                StdCrate::Alloc,
                &["collections", "vec_deque", "spec_extend", "SpecExtend"],
            ),
            Some((StdCrate::Alloc, &["collections", "vec_deque", "VecDeque"])),
            (StdCrate::Core, &["iter", "traits", "marker", "TrustedLen"]),
        ),
        "spec_extend",
    ),
    (
        StdRow::VecDequeReserve,
        None,
        Owner::Type(StdCrate::Alloc, &["collections", "vec_deque", "VecDeque"]),
        "reserve",
    ),
    // A sort's comparison that isn't a total order, its merge's check.
    (
        StdRow::SortTotalOrder,
        None,
        Owner::Free(
            StdCrate::Core,
            &["slice", "sort", "shared", "smallsort", "panic_on_ord_violation"],
        ),
        "",
    ),
    // std's `Ord::clamp` default, its `assert!(min <= max)`, which doesn't
    // track its caller as integers' and a `char`'s do.
    (
        StdRow::OrdClamp,
        None,
        Owner::Default(StdCrate::Core, &["cmp", "Ord"]),
        "clamp",
    ),
    // Of rustc's shims a `RefCell`'s clone, tracking its caller, raises its
    // panic at (#2471): a tuple's `Clone::clone` and a function's
    // `FnMut::call_mut`. A std type's own clone's are `clonedAt`'s, read of
    // the impl rustc resolves it to (#2509).
    (
        StdRow::TupleClone,
        None,
        Owner::Declared(StdCrate::Core, &["clone", "Clone"]),
        "clone",
    ),
    // A shim's call of a function value given as an `FnOnce`, a chain's
    // `last`'s `Iterator::last` given to `Option::and_then` (#2488).
    (
        StdRow::FnOnceCall,
        None,
        Owner::Declared(StdCrate::Core, &["ops", "function", "FnOnce"]),
        "call_once",
    ),
    (
        StdRow::RefCellEq,
        None,
        Owner::Impl {
            trait_: Trait::Lang(LangItem::PartialEq),
            of: (StdCrate::Core, &["cell", "RefCell"]),
        },
        "eq",
    ),
    (
        StdRow::NextMultipleOf,
        Some("u8"),
        Owner::Primitive(SimplifiedType::Uint(UintTy::U8)),
        "next_multiple_of",
    ),
    (
        StdRow::NextMultipleOf,
        Some("u16"),
        Owner::Primitive(SimplifiedType::Uint(UintTy::U16)),
        "next_multiple_of",
    ),
    (
        StdRow::NextMultipleOf,
        Some("u32"),
        Owner::Primitive(SimplifiedType::Uint(UintTy::U32)),
        "next_multiple_of",
    ),
    (
        StdRow::NextMultipleOf,
        Some("u64"),
        Owner::Primitive(SimplifiedType::Uint(UintTy::U64)),
        "next_multiple_of",
    ),
    (
        StdRow::NextMultipleOf,
        Some("u128"),
        Owner::Primitive(SimplifiedType::Uint(UintTy::U128)),
        "next_multiple_of",
    ),
    (
        StdRow::NextMultipleOf,
        Some("usize"),
        Owner::Primitive(SimplifiedType::Uint(UintTy::Usize)),
        "next_multiple_of",
    ),
    (
        StdRow::FromStrRadix,
        Some("signed"),
        Owner::Primitive(SimplifiedType::Int(IntTy::I8)),
        "from_ascii_bytes_radix_impl",
    ),
    (
        StdRow::FromStrRadix,
        Some("signed"),
        Owner::Primitive(SimplifiedType::Int(IntTy::I16)),
        "from_ascii_bytes_radix_impl",
    ),
    (
        StdRow::FromStrRadix,
        Some("signed"),
        Owner::Primitive(SimplifiedType::Int(IntTy::I32)),
        "from_ascii_bytes_radix_impl",
    ),
    (
        StdRow::FromStrRadix,
        Some("signed"),
        Owner::Primitive(SimplifiedType::Int(IntTy::I64)),
        "from_ascii_bytes_radix_impl",
    ),
    (
        StdRow::FromStrRadix,
        Some("signed"),
        Owner::Primitive(SimplifiedType::Int(IntTy::I128)),
        "from_ascii_bytes_radix_impl",
    ),
    (
        StdRow::FromStrRadix,
        Some("signed"),
        Owner::Primitive(SimplifiedType::Int(IntTy::Isize)),
        "from_ascii_bytes_radix_impl",
    ),
    (
        StdRow::FromStrRadix,
        Some("unsigned"),
        Owner::Primitive(SimplifiedType::Uint(UintTy::U8)),
        "from_ascii_bytes_radix_impl",
    ),
    (
        StdRow::FromStrRadix,
        Some("unsigned"),
        Owner::Primitive(SimplifiedType::Uint(UintTy::U16)),
        "from_ascii_bytes_radix_impl",
    ),
    (
        StdRow::FromStrRadix,
        Some("unsigned"),
        Owner::Primitive(SimplifiedType::Uint(UintTy::U32)),
        "from_ascii_bytes_radix_impl",
    ),
    (
        StdRow::FromStrRadix,
        Some("unsigned"),
        Owner::Primitive(SimplifiedType::Uint(UintTy::U64)),
        "from_ascii_bytes_radix_impl",
    ),
    (
        StdRow::FromStrRadix,
        Some("unsigned"),
        Owner::Primitive(SimplifiedType::Uint(UintTy::U128)),
        "from_ascii_bytes_radix_impl",
    ),
    (
        StdRow::FromStrRadix,
        Some("unsigned"),
        Owner::Primitive(SimplifiedType::Uint(UintTy::Usize)),
        "from_ascii_bytes_radix_impl",
    ),
    (
        StdRow::WriteCloneOfSlice,
        None,
        Owner::Primitive(SimplifiedType::Slice),
        "write_clone_of_slice",
    ),
    (
        StdRow::WriteCopyOfSlice,
        None,
        Owner::Primitive(SimplifiedType::Slice),
        "write_copy_of_slice",
    ),
    (
        StdRow::ExactSizeLen,
        None,
        Owner::Default(StdCrate::Core, &["iter", "ExactSizeIterator"]),
        "len",
    ),
    (
        StdRow::StepForward,
        None,
        Owner::Default(StdCrate::Core, &["iter", "Step"]),
        "forward",
    ),
    (
        StdRow::ToString,
        None,
        Owner::Blanket(StdCrate::Alloc, &["string", "SpecToString"]),
        "spec_to_string",
    ),
    (
        StdRow::Format,
        None,
        Owner::Free(StdCrate::Alloc, &["fmt", "format"]),
        "",
    ),
    (
        StdRow::PrintFailed,
        None,
        Owner::Free(StdCrate::Core, &["io", "write", "default_write_fmt"]),
        "",
    ),
    (
        StdRow::BufReaderStreamPosition,
        None,
        Owner::Impl {
            trait_: Trait::Path(StdCrate::Alloc, &["io", "Seek"]),
            of: (StdCrate::Alloc, &["io", "BufReader"]),
        },
        "stream_position",
    ),
    (
        StdRow::LazyLockPoisoned,
        None,
        Owner::Free(StdCrate::Std, &["sync", "lazy_lock", "panic_poisoned"]),
        "",
    ),
    (
        StdRow::PathExtension,
        None,
        Owner::Free(StdCrate::Std, &["path", "validate_extension"]),
        "",
    ),
    (
        StdRow::VecDrain,
        None,
        Owner::Type(StdCrate::Alloc, &["vec", "Vec"]),
        "drain",
    ),
    (
        StdRow::VecExtendFromWithin,
        None,
        Owner::Type(StdCrate::Alloc, &["vec", "Vec"]),
        "extend_from_within",
    ),
    (
        StdRow::VecExtractIf,
        None,
        Owner::Type(StdCrate::Alloc, &["vec", "ExtractIf"]),
        "new",
    ),
    (
        StdRow::VecDequeDrain,
        None,
        Owner::Type(StdCrate::Alloc, &["collections", "vec_deque", "VecDeque"]),
        "drain",
    ),
    (
        StdRow::VecDequeRange,
        None,
        Owner::Type(StdCrate::Alloc, &["collections", "vec_deque", "VecDeque"]),
        "slice_ranges",
    ),
    (
        StdRow::VecDequeSplitOff,
        None,
        Owner::Type(StdCrate::Alloc, &["collections", "vec_deque", "VecDeque"]),
        "split_off",
    ),
    (
        StdRow::ByteSliceConsume,
        None,
        Owner::ForByteSlice(StdCrate::Alloc, &["io", "BufRead"]),
        "consume",
    ),
    (
        StdRow::WrappingOp,
        None,
        Owner::Impl {
            trait_: Trait::Lang(LangItem::Div),
            of: (StdCrate::Core, &["num", "Wrapping"]),
        },
        "div",
    ),
    (
        StdRow::SaturatingOp,
        None,
        Owner::Impl {
            trait_: Trait::Lang(LangItem::Div),
            of: (StdCrate::Core, &["num", "Saturating"]),
        },
        "div",
    ),
];
