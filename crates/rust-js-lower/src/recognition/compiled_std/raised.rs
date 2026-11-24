//! Where std's own functions raise the panics std's hook names a line of
//! std's own of (#2292): `VecDeque::swap`'s `assert!(j < self.len())`,
//! `/rustc/<commit>/library/alloc/src/collections/vec_deque/mod.rs:1063:9`,
//! where a caller's own `v[9]` names the caller's. Read by `bun run std`
//! (`--std-raised`) from each one's MIR in std's own build, of a `core` and
//! an `alloc` built of the toolchain's source with every body's MIR, for
//! the target rust-js compiles for, wasm32's (ADR 0562), each site as
//! rustc's `caller_location` places it, and written as the runtime's
//! `$stdSites`, each a constant of the toolchain, whatever calls it: a
//! helper that raises one reads it (ADR 0262's exception, the user's
//! decision: always under `/rustc/<commit>/`).

use super::*;
use rust_js_program::collections::{IndexMap, IndexSet};
use rustc_hir::attrs::lang_items::LangItem;
use rustc_middle::mir;
use rustc_middle::ty::fast_reject::SimplifiedType;
use rustc_middle::ty::{FloatTy, IntTy, UintTy};

/// A trait: by its lang item, as a derive macro has its name, or at its
/// path, of one that isn't one.
#[derive(Clone, Copy, Debug)]
enum Trait {
    Lang(LangItem),
    Path(StdCrate, &'static [&'static str]),
}

/// Whose a function is: a type's own, at its path in its crate; a
/// primitive's own; a trait's impl, the trait by its lang item, as a
/// derive macro has its name, for a type at its path; or a free function,
/// at its path, its name the path's last.
#[derive(Clone, Copy, Debug)]
enum Owner {
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

/// `Repeat<u8>`, of which the types `Owner::Resolved` resolves for are made.
fn a_repeat(tcx: TyCtxt<'_>) -> Option<Ty<'_>> {
    let repeat = registry::by_path(tcx, StdCrate::Core, &["iter", "sources", "repeat", "Repeat"])?;
    Some(Ty::new_adt(
        tcx,
        tcx.adt_def(repeat),
        tcx.mk_args(&[tcx.types.u8.into()]),
    ))
}

/// `Box<Repeat<u8>>`.
fn box_of_a_repeat(tcx: TyCtxt<'_>) -> Option<Ty<'_>> {
    Some(Ty::new_box(tcx, a_repeat(tcx)?))
}

/// A flatten's own, `FlattenCompat<Empty<Repeat<u8>>, Repeat<u8>>`.
fn flatten_of_repeats(tcx: TyCtxt<'_>) -> Option<Ty<'_>> {
    let repeat = a_repeat(tcx)?;
    let empty = registry::by_path(tcx, StdCrate::Core, &["iter", "sources", "empty", "Empty"])?;
    let flatten = registry::by_path(tcx, StdCrate::Core, &["iter", "adapters", "flatten", "FlattenCompat"])?;
    let items = Ty::new_adt(tcx, tcx.adt_def(empty), tcx.mk_args(&[repeat.into()]));
    Some(Ty::new_adt(
        tcx,
        tcx.adt_def(flatten),
        tcx.mk_args(&[items.into(), repeat.into()]),
    ))
}

/// Each std adapter's `count` or `last` that passes the step on to what it
/// holds, by its name in `$stdSites`, whose sites are where it calls it:
/// a `Repeat`'s own, which panics naming its caller, as std's tracks it,
/// so names the adapter's line, the nearest frame that doesn't (#2488).
/// Each names the adapters of std's step catalog (`std_steps`) it's of; and
/// `bun run std` holds them to it both ways (`held_steps_checked`).
const HELD_STEPS: &[(&str, &[&str], Owner, &str)] = &[
    (
        "flattenCount",
        &["Flatten", "FlatMap"],
        Owner::Resolved(
            (StdCrate::Core, &["iter", "traits", "iterator", "Iterator"]),
            flatten_of_repeats,
        ),
        "count",
    ),
    (
        "flattenLast",
        &["Flatten", "FlatMap"],
        Owner::Resolved(
            (StdCrate::Core, &["iter", "traits", "iterator", "Iterator"]),
            flatten_of_repeats,
        ),
        "last",
    ),
    (
        "boxLast",
        &["Box<I>"],
        Owner::Resolved((StdCrate::Alloc, &["boxed", "iter", "BoxIter"]), box_of_a_repeat),
        "last",
    ),
    (
        "skipCount",
        &["Skip"],
        Owner::Impl {
            trait_: Trait::Path(StdCrate::Core, &["iter", "traits", "iterator", "Iterator"]),
            of: (StdCrate::Core, &["iter", "adapters", "skip", "Skip"]),
        },
        "count",
    ),
    (
        "skipLast",
        &["Skip"],
        Owner::Impl {
            trait_: Trait::Path(StdCrate::Core, &["iter", "traits", "iterator", "Iterator"]),
            of: (StdCrate::Core, &["iter", "adapters", "skip", "Skip"]),
        },
        "last",
    ),
    (
        "enumerateCount",
        &["Enumerate"],
        Owner::Impl {
            trait_: Trait::Path(StdCrate::Core, &["iter", "traits", "iterator", "Iterator"]),
            of: (StdCrate::Core, &["iter", "adapters", "enumerate", "Enumerate"]),
        },
        "count",
    ),
    (
        "chainCount",
        &["Chain"],
        Owner::Impl {
            trait_: Trait::Path(StdCrate::Core, &["iter", "traits", "iterator", "Iterator"]),
            of: (StdCrate::Core, &["iter", "adapters", "chain", "Chain"]),
        },
        "count",
    ),
    (
        "copiedCount",
        &["Copied"],
        Owner::Impl {
            trait_: Trait::Path(StdCrate::Core, &["iter", "traits", "iterator", "Iterator"]),
            of: (StdCrate::Core, &["iter", "adapters", "copied", "Copied"]),
        },
        "count",
    ),
    (
        "copiedLast",
        &["Copied"],
        Owner::Impl {
            trait_: Trait::Path(StdCrate::Core, &["iter", "traits", "iterator", "Iterator"]),
            of: (StdCrate::Core, &["iter", "adapters", "copied", "Copied"]),
        },
        "last",
    ),
];

/// Of the catalog's adapters whose `count` or `last` asks the same step of
/// what it holds, those no `HELD_STEPS` row is of, and why: a chain's
/// `last`, which gives `Iterator::last` to `Option::and_then`, raised at
/// the `FnOnce` shim's row, `fnOnceCall`; and a `fuse()`'s and a
/// `peekable()`'s, made of std's compiled adapter, whose own body gives
/// std's line (#2488).
const HELD_STEPS_ELSEWHERE: &[(&str, &str)] = &[
    ("Chain", "last"),
    ("Fuse", "count"),
    ("Fuse", "last"),
    ("Peekable", "count"),
    ("Peekable", "last"),
];

/// That `HELD_STEPS` is std's step catalog's, both ways: each row's
/// adapters' step asks the same step of what they hold, and each of the
/// catalog's that does is a row's or `HELD_STEPS_ELSEWHERE`'s, so a step a
/// new toolchain's adapter passes on is said, never left naming its
/// caller's line (#2488).
fn held_steps_checked(rows: &[(&str, &[&str], Owner, &str)], elsewhere: &[(&str, &str)]) -> Result<(), String> {
    use crate::recognition::std_steps;
    let asks_the_same = |name: &str, step: &str| std_steps::asks(name, step).iter().any(|asked| asked.step == step);
    for &(row, names, _, step) in rows {
        if let Some(name) = names.iter().find(|name| !asks_the_same(name, step)) {
            return Err(format!(
                "`{row}`: std's catalog says `{name}`'s `{step}` asks no `{step}` of what it holds"
            ));
        }
    }
    for name in std_steps::adapters() {
        for step in ["count", "last"] {
            let named = rows
                .iter()
                .any(|&(_, names, _, of)| of == step && names.contains(&name))
                || elsewhere.contains(&(name, step));
            if asks_the_same(name, step) && !named {
                return Err(format!(
                    "std's catalog says `{name}`'s `{step}` asks `{step}` of what it holds, which no `HELD_STEPS` row is of"
                ));
            }
        }
    }
    Ok(())
}

/// Each site of std's panics a helper raises, by its name in `$stdSites`,
/// and a type's where a helper serves several (`u8`), or a kind of type's
/// whose sites are one (`signed`): whose function holds
/// it, and its name, of a free function none, its path's last. A
/// function's sites are in the order they're written.
const RAISED: &[(&str, Option<&str>, Owner, &str)] = &[
    (
        "vecDequeSwap",
        None,
        Owner::Type(StdCrate::Alloc, &["collections", "vec_deque", "VecDeque"]),
        "swap",
    ),
    (
        "vecDequeInsert",
        None,
        Owner::Type(StdCrate::Alloc, &["collections", "vec_deque", "VecDeque"]),
        "insert_mut",
    ),
    (
        "vecDequeRotateLeft",
        None,
        Owner::Type(StdCrate::Alloc, &["collections", "vec_deque", "VecDeque"]),
        "rotate_left",
    ),
    (
        "vecDequeRotateRight",
        None,
        Owner::Type(StdCrate::Alloc, &["collections", "vec_deque", "VecDeque"]),
        "rotate_right",
    ),
    (
        "swapRemove",
        None,
        Owner::Type(StdCrate::Alloc, &["vec", "Vec"]),
        "swap_remove",
    ),
    (
        "capacityOverflow",
        None,
        Owner::Free(StdCrate::Alloc, &["raw_vec", "capacity_overflow"]),
        "",
    ),
    // A panic that can't unwind, which aborts: std's hook names the line in
    // `core::panicking` its message is raised at, of a function that can't
    // unwind's, `panic_nounwind`'s, and of a drop's panic as another
    // unwinds, `panic_nounwind_nobacktrace`'s (`$abort`, #2452).
    (
        "panicNounwind",
        None,
        Owner::Free(StdCrate::Core, &["panicking", "panic_nounwind"]),
        "",
    ),
    (
        "panicNounwindNobacktrace",
        None,
        Owner::Free(StdCrate::Core, &["panicking", "panic_nounwind_nobacktrace"]),
        "",
    ),
    ("strSplitAt", None, Owner::Primitive(SimplifiedType::Str), "split_at"),
    // A `HashMap`'s `get_disjoint_mut` of two keys of one entry: hashbrown's
    // table's, vendored, named under its own build's registry (#2325).
    (
        "mapGetDisjointMut",
        None,
        Owner::Vendored(
            &["raw", "RawTable"],
            (StdCrate::Std, &["collections", "hash", "map", "HashMap"], "base"),
        ),
        "get_disjoint_mut",
    ),
    (
        "strSplitAtMut",
        None,
        Owner::Primitive(SimplifiedType::Str),
        "split_at_mut",
    ),
    // A hash table's room past what its buckets can count: hashbrown's
    // `Fallibility::capacity_overflow`, its `panic!` (#2487).
    (
        "hashTableCapacityOverflow",
        None,
        Owner::Vendored(
            &["raw", "Fallibility"],
            (StdCrate::Std, &["collections", "hash", "map", "HashMap"], "base"),
        ),
        "capacity_overflow",
    ),
    (
        "rotateLeft",
        None,
        Owner::Primitive(SimplifiedType::Slice),
        "rotate_left",
    ),
    (
        "rotateRight",
        None,
        Owner::Primitive(SimplifiedType::Slice),
        "rotate_right",
    ),
    (
        "selectNth",
        None,
        Owner::Free(StdCrate::Core, &["slice", "sort", "select", "partition_at_index"]),
        "",
    ),
    (
        "clamp",
        Some("f64"),
        Owner::Primitive(SimplifiedType::Float(FloatTy::F64)),
        "clamp",
    ),
    (
        "clamp",
        Some("f32"),
        Owner::Primitive(SimplifiedType::Float(FloatTy::F32)),
        "clamp",
    ),
    (
        "fromDigit",
        None,
        Owner::Free(StdCrate::Core, &["char", "convert", "from_digit"]),
        "",
    ),
    (
        "encodeUtf8",
        None,
        Owner::Free(StdCrate::Core, &["char", "encode_utf8_raw"]),
        "",
    ),
    (
        "encodeUtf16",
        None,
        Owner::Free(StdCrate::Core, &["char", "encode_utf16_raw"]),
        "",
    ),
    ("stepBy", None, Owner::Type(StdCrate::Core, &["iter", "StepBy"]), "new"),
    (
        "durationNew",
        None,
        Owner::Type(StdCrate::Core, &["time", "Duration"]),
        "new",
    ),
    (
        "durationAdd",
        None,
        Owner::Impl {
            trait_: Trait::Lang(LangItem::Add),
            of: (StdCrate::Core, &["time", "Duration"]),
        },
        "add",
    ),
    (
        "durationSub",
        None,
        Owner::Impl {
            trait_: Trait::Lang(LangItem::Sub),
            of: (StdCrate::Core, &["time", "Duration"]),
        },
        "sub",
    ),
    (
        "onceCellReentrant",
        None,
        Owner::Type(StdCrate::Core, &["cell", "OnceCell"]),
        "try_init",
    ),
    (
        "lazyCellPoisoned",
        None,
        Owner::Free(StdCrate::Core, &["cell", "lazy", "panic_poisoned"]),
        "",
    ),
    (
        "btreeRange",
        None,
        Owner::Type(StdCrate::Alloc, &["collections", "btree", "node", "NodeRef"]),
        "search_tree_for_bifurcation",
    ),
    // `RefCell::take`'s borrow, its `replace`'s call, and `swap`'s two,
    // `self`'s and `other`'s.
    (
        "refCellTake",
        None,
        Owner::Type(StdCrate::Core, &["cell", "RefCell"]),
        "take",
    ),
    (
        "refCellSwap",
        None,
        Owner::Type(StdCrate::Core, &["cell", "RefCell"]),
        "swap",
    ),
    // A thread-local `RefCell`'s: each its `with`'s call, then its borrow.
    (
        "localKeyWithBorrow",
        None,
        Owner::TypeOver(
            StdCrate::Std,
            &["thread", "local", "LocalKey"],
            (StdCrate::Core, &["cell", "RefCell"]),
        ),
        "with_borrow",
    ),
    (
        "localKeyWithBorrowMut",
        None,
        Owner::TypeOver(
            StdCrate::Std,
            &["thread", "local", "LocalKey"],
            (StdCrate::Core, &["cell", "RefCell"]),
        ),
        "with_borrow_mut",
    ),
    (
        "localKeySet",
        None,
        Owner::TypeOver(
            StdCrate::Std,
            &["thread", "local", "LocalKey"],
            (StdCrate::Core, &["cell", "RefCell"]),
        ),
        "set",
    ),
    (
        "localKeyReplace",
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
        "vecFromTrustedLen",
        None,
        Owner::Bounded(
            (StdCrate::Alloc, &["vec", "spec_from_iter_nested", "SpecFromIterNested"]),
            Some((StdCrate::Alloc, &["vec", "Vec"])),
            (StdCrate::Core, &["iter", "traits", "marker", "TrustedLen"]),
        ),
        "from_iter",
    ),
    (
        "rcFromTrustedLen",
        None,
        Owner::Bounded(
            (StdCrate::Alloc, &["rc", "ToRcSlice"]),
            None,
            (StdCrate::Core, &["iter", "traits", "marker", "TrustedLen"]),
        ),
        "to_rc_slice",
    ),
    (
        "arcFromTrustedLen",
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
    ("sliceRepeat", None, Owner::Primitive(SimplifiedType::Slice), "repeat"),
    // Where `Rc::<[T]>`'s and `Arc::<[T]>`'s slice is allocated, its
    // `Layout::array::<T>(len).unwrap()`, which a collect past what a
    // `Layout` holds panics at (#2489).
    (
        "rcSliceLayout",
        None,
        Owner::Type(StdCrate::Alloc, &["rc", "Rc"]),
        "allocate_for_slice",
    ),
    (
        "arcSliceLayout",
        None,
        Owner::Type(StdCrate::Alloc, &["sync", "Arc"]),
        "allocate_for_slice",
    ),
    (
        "vecExtendTrusted",
        None,
        Owner::Type(StdCrate::Alloc, &["vec", "Vec"]),
        "extend_trusted",
    ),
    (
        "vecDequeExtendTrusted",
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
        "vecDequeReserve",
        None,
        Owner::Type(StdCrate::Alloc, &["collections", "vec_deque", "VecDeque"]),
        "reserve",
    ),
    // A sort's comparison that isn't a total order, its merge's check.
    (
        "sortTotalOrder",
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
        "ordClamp",
        None,
        Owner::Default(StdCrate::Core, &["cmp", "Ord"]),
        "clamp",
    ),
    // Of rustc's shims a `RefCell`'s clone, tracking its caller, raises its
    // panic at (#2471): a tuple's `Clone::clone` and a function's
    // `FnMut::call_mut`. A std type's own clone's are `clonedAt`'s, read of
    // the impl rustc resolves it to (#2509).
    (
        "tupleClone",
        None,
        Owner::Declared(StdCrate::Core, &["clone", "Clone"]),
        "clone",
    ),
    // A shim's call of a function value given as an `FnOnce`, a chain's
    // `last`'s `Iterator::last` given to `Option::and_then` (#2488).
    (
        "fnOnceCall",
        None,
        Owner::Declared(StdCrate::Core, &["ops", "function", "FnOnce"]),
        "call_once",
    ),
    (
        "fnMutCall",
        None,
        Owner::Declared(StdCrate::Core, &["ops", "function", "FnMut"]),
        "call_mut",
    ),
    (
        "refCellEq",
        None,
        Owner::Impl {
            trait_: Trait::Lang(LangItem::PartialEq),
            of: (StdCrate::Core, &["cell", "RefCell"]),
        },
        "eq",
    ),
    (
        "nextMultipleOf",
        Some("u8"),
        Owner::Primitive(SimplifiedType::Uint(UintTy::U8)),
        "next_multiple_of",
    ),
    (
        "nextMultipleOf",
        Some("u16"),
        Owner::Primitive(SimplifiedType::Uint(UintTy::U16)),
        "next_multiple_of",
    ),
    (
        "nextMultipleOf",
        Some("u32"),
        Owner::Primitive(SimplifiedType::Uint(UintTy::U32)),
        "next_multiple_of",
    ),
    (
        "nextMultipleOf",
        Some("u64"),
        Owner::Primitive(SimplifiedType::Uint(UintTy::U64)),
        "next_multiple_of",
    ),
    (
        "nextMultipleOf",
        Some("u128"),
        Owner::Primitive(SimplifiedType::Uint(UintTy::U128)),
        "next_multiple_of",
    ),
    (
        "nextMultipleOf",
        Some("usize"),
        Owner::Primitive(SimplifiedType::Uint(UintTy::Usize)),
        "next_multiple_of",
    ),
    (
        "fromStrRadix",
        Some("signed"),
        Owner::Primitive(SimplifiedType::Int(IntTy::I8)),
        "from_ascii_bytes_radix_impl",
    ),
    (
        "fromStrRadix",
        Some("signed"),
        Owner::Primitive(SimplifiedType::Int(IntTy::I16)),
        "from_ascii_bytes_radix_impl",
    ),
    (
        "fromStrRadix",
        Some("signed"),
        Owner::Primitive(SimplifiedType::Int(IntTy::I32)),
        "from_ascii_bytes_radix_impl",
    ),
    (
        "fromStrRadix",
        Some("signed"),
        Owner::Primitive(SimplifiedType::Int(IntTy::I64)),
        "from_ascii_bytes_radix_impl",
    ),
    (
        "fromStrRadix",
        Some("signed"),
        Owner::Primitive(SimplifiedType::Int(IntTy::I128)),
        "from_ascii_bytes_radix_impl",
    ),
    (
        "fromStrRadix",
        Some("signed"),
        Owner::Primitive(SimplifiedType::Int(IntTy::Isize)),
        "from_ascii_bytes_radix_impl",
    ),
    (
        "fromStrRadix",
        Some("unsigned"),
        Owner::Primitive(SimplifiedType::Uint(UintTy::U8)),
        "from_ascii_bytes_radix_impl",
    ),
    (
        "fromStrRadix",
        Some("unsigned"),
        Owner::Primitive(SimplifiedType::Uint(UintTy::U16)),
        "from_ascii_bytes_radix_impl",
    ),
    (
        "fromStrRadix",
        Some("unsigned"),
        Owner::Primitive(SimplifiedType::Uint(UintTy::U32)),
        "from_ascii_bytes_radix_impl",
    ),
    (
        "fromStrRadix",
        Some("unsigned"),
        Owner::Primitive(SimplifiedType::Uint(UintTy::U64)),
        "from_ascii_bytes_radix_impl",
    ),
    (
        "fromStrRadix",
        Some("unsigned"),
        Owner::Primitive(SimplifiedType::Uint(UintTy::U128)),
        "from_ascii_bytes_radix_impl",
    ),
    (
        "fromStrRadix",
        Some("unsigned"),
        Owner::Primitive(SimplifiedType::Uint(UintTy::Usize)),
        "from_ascii_bytes_radix_impl",
    ),
    (
        "writeCloneOfSlice",
        None,
        Owner::Primitive(SimplifiedType::Slice),
        "write_clone_of_slice",
    ),
    (
        "writeCopyOfSlice",
        None,
        Owner::Primitive(SimplifiedType::Slice),
        "write_copy_of_slice",
    ),
    (
        "exactSizeLen",
        None,
        Owner::Default(StdCrate::Core, &["iter", "ExactSizeIterator"]),
        "len",
    ),
    (
        "stepForward",
        None,
        Owner::Default(StdCrate::Core, &["iter", "Step"]),
        "forward",
    ),
    (
        "toString",
        None,
        Owner::Blanket(StdCrate::Alloc, &["string", "SpecToString"]),
        "spec_to_string",
    ),
    ("format", None, Owner::Free(StdCrate::Alloc, &["fmt", "format"]), ""),
    (
        "printFailed",
        None,
        Owner::Free(StdCrate::Core, &["io", "write", "default_write_fmt"]),
        "",
    ),
    (
        "bufReaderStreamPosition",
        None,
        Owner::Impl {
            trait_: Trait::Path(StdCrate::Alloc, &["io", "Seek"]),
            of: (StdCrate::Alloc, &["io", "BufReader"]),
        },
        "stream_position",
    ),
    (
        "lazyLockPoisoned",
        None,
        Owner::Free(StdCrate::Std, &["sync", "lazy_lock", "panic_poisoned"]),
        "",
    ),
    (
        "pathExtension",
        None,
        Owner::Free(StdCrate::Std, &["path", "validate_extension"]),
        "",
    ),
    ("vecDrain", None, Owner::Type(StdCrate::Alloc, &["vec", "Vec"]), "drain"),
    (
        "vecExtendFromWithin",
        None,
        Owner::Type(StdCrate::Alloc, &["vec", "Vec"]),
        "extend_from_within",
    ),
    (
        "vecExtractIf",
        None,
        Owner::Type(StdCrate::Alloc, &["vec", "ExtractIf"]),
        "new",
    ),
    (
        "vecDequeDrain",
        None,
        Owner::Type(StdCrate::Alloc, &["collections", "vec_deque", "VecDeque"]),
        "drain",
    ),
    (
        "vecDequeRange",
        None,
        Owner::Type(StdCrate::Alloc, &["collections", "vec_deque", "VecDeque"]),
        "slice_ranges",
    ),
    (
        "vecDequeSplitOff",
        None,
        Owner::Type(StdCrate::Alloc, &["collections", "vec_deque", "VecDeque"]),
        "split_off",
    ),
    (
        "byteSliceConsume",
        None,
        Owner::ForByteSlice(StdCrate::Alloc, &["io", "BufRead"]),
        "consume",
    ),
    (
        "wrappingOp",
        None,
        Owner::Impl {
            trait_: Trait::Lang(LangItem::Div),
            of: (StdCrate::Core, &["num", "Wrapping"]),
        },
        "div",
    ),
    (
        "saturatingOp",
        None,
        Owner::Impl {
            trait_: Trait::Lang(LangItem::Div),
            of: (StdCrate::Core, &["num", "Saturating"]),
        },
        "div",
    ),
];

/// The function `owner`'s `name` is, where std has it.
fn function(tcx: TyCtxt<'_>, owner: Owner, name: &str) -> Option<DefId> {
    let impls: Vec<DefId> = match owner {
        Owner::Type(krate, path) => tcx.inherent_impls(registry::by_path(tcx, krate, path)?).to_vec(),
        Owner::TypeOver(krate, path, over) => {
            let over = registry::by_path(tcx, over.0, over.1)?;
            (tcx.inherent_impls(registry::by_path(tcx, krate, path)?).iter().copied())
                .filter(|&imp| match tcx.type_of(imp).skip_binder().kind() {
                    ty::Adt(_, args) => {
                        matches!(args.types().next().map(|of| of.kind()), Some(ty::Adt(adt, _)) if adt.did() == over)
                    }
                    _ => false,
                })
                .collect()
        }
        Owner::Primitive(kind) => tcx.incoherent_impls(kind).to_vec(),
        Owner::Vendored(path, (krate, of, field)) => {
            let of = registry::by_path(tcx, krate, of)?;
            let field = (tcx.adt_def(of).all_fields()).find(|each| each.name.as_str() == field)?;
            let ty::Adt(held, _) = tcx.type_of(field.did).skip_binder().kind() else {
                return None;
            };
            let (_, ty) = registry::by_path_in(tcx, held.did().krate, path)?;
            tcx.inherent_impls(ty).to_vec()
        }
        Owner::Bounded(trait_, of, bound) => {
            let trait_ = registry::by_path(tcx, trait_.0, trait_.1)?;
            let of = match of {
                Some(of) => Some(registry::by_path(tcx, of.0, of.1)?),
                None => None,
            };
            let bound = registry::by_path(tcx, bound.0, bound.1)?;
            let impls: Vec<DefId> = match of {
                Some(of) => (tcx.all_impls(trait_))
                    .filter(|&imp| matches!(tcx.type_of(imp).skip_binder().kind(), ty::Adt(adt, _) if adt.did() == of))
                    .collect(),
                None => tcx.trait_impls_of(trait_).blanket_impls().to_vec(),
            };
            (impls.into_iter())
                .filter(|&imp| {
                    tcx.param_env(imp)
                        .caller_bounds()
                        .any(|clause| clause.as_trait_clause().is_some_and(|clause| clause.def_id() == bound))
                })
                .collect()
        }
        Owner::Free(krate, path) => return registry::by_path(tcx, krate, path),
        Owner::Resolved((krate, path), made) => {
            let trait_ = registry::by_path(tcx, krate, path)?;
            let name = rustc_span::Symbol::intern(name);
            let method = (tcx.associated_items(trait_).filter_by_name_unhygienic(name))
                .next()?
                .def_id;
            let env = ty::TypingEnv::fully_monomorphized();
            let instance = crate::traits::resolve(tcx, env, method, tcx.mk_args(&[made(tcx)?.into()])).ok()??;
            return Some(instance.def_id());
        }
        Owner::Default(krate, path) | Owner::Declared(krate, path) => vec![registry::by_path(tcx, krate, path)?],
        Owner::ForByteSlice(krate, path) => {
            let trait_ = registry::by_path(tcx, krate, path)?;
            (tcx.all_impls(trait_))
                .filter(|&imp| {
                    let of = tcx.type_of(imp).skip_binder();
                    matches!(of.kind(), ty::Ref(_, inner, _) if matches!(inner.kind(), ty::Slice(item) if matches!(item.kind(), ty::Uint(ty::UintTy::U8))))
                })
                .collect()
        }
        Owner::Blanket(krate, path) => {
            let trait_ = registry::by_path(tcx, krate, path)?;
            tcx.trait_impls_of(trait_).blanket_impls().to_vec()
        }
        Owner::Impl { trait_, of } => {
            let trait_ = match trait_ {
                Trait::Lang(item) => tcx.lang_items().get(item)?,
                Trait::Path(krate, path) => registry::by_path(tcx, krate, path)?,
            };
            let of = registry::by_path(tcx, of.0, of.1)?;
            (tcx.all_impls(trait_))
                .filter(|&imp| matches!(tcx.type_of(imp).skip_binder().kind(), ty::Adt(adt, _) if adt.did() == of))
                .collect()
        }
    };
    let name = rustc_span::Symbol::intern(name);
    (impls.into_iter())
        .flat_map(|imp| {
            tcx.associated_items(imp)
                .filter_by_name_unhygienic(name)
                .map(|item| item.def_id)
        })
        .find(|&id| matches!(tcx.def_kind(id), DefKind::AssocFn))
}

/// Where `info`'s call or check raises a panic, as rustc's codegen places
/// it (`get_caller_location`): up through each scope inlined of a function
/// that tracks its caller, to the call of the outermost, at its outermost
/// macro's call. `None` of one inside a body inlined of another function,
/// `is_char_boundary`'s bounds check in `split_at_mut`: that function's,
/// not this one's; but of one inlined of a closure of `root`'s own,
/// through `Result::map`'s, `root`'s, where its closure writes it.
fn site(tcx: TyCtxt<'_>, root: DefId, body: &mir::Body<'_>, info: mir::SourceInfo) -> Option<(String, usize, usize)> {
    located_through(tcx, Some(root), body, info)
}

/// `site`'s, of `root`'s own; of none, codegen's whatever body it's
/// inlined of: at the first inlined function that doesn't track its
/// caller, an `Option`'s `clone` inlined into an iterator's, that one's
/// line, as the frame a `RefCell`'s panic names (#2509).
pub(super) fn located_through(
    tcx: TyCtxt<'_>,
    root: Option<DefId>,
    body: &mir::Body<'_>,
    info: mir::SourceInfo,
) -> Option<(String, usize, usize)> {
    let mut span = info.span;
    let mut scope = info.scope;
    loop {
        let data = &body.source_scopes[scope];
        if let Some((callee, call)) = data.inlined {
            match (
                tracks_caller(tcx, callee.def_id()),
                root.is_none_or(|root| nested_in(tcx, callee.def_id(), root)),
            ) {
                (true, _) => span = call,
                (false, true) => break,
                (false, false) => return None,
            }
        }
        match data.inlined_parent_scope {
            Some(parent) => scope = parent,
            None => break,
        }
    }
    Some(located(tcx, span))
}

/// `span`'s site, `library/..`, line and column: at its outermost macro's
/// call, as rustc's `caller_location` places it.
pub(super) fn located(tcx: TyCtxt<'_>, span: rustc_span::Span) -> (String, usize, usize) {
    let topmost = span.ctxt().outer_expn().expansion_cause().unwrap_or(span);
    let at = tcx.sess.source_map().lookup_char_pos(topmost.lo());
    let file = at
        .file
        .name
        .display(rustc_span::RemapPathScopeComponents::MACRO)
        .to_string_lossy()
        .into_owned();
    // Of std's source, from its `library/`, which `bun run std` places
    // under the toolchain's `/rustc/<commit>/`, as its shipped std names it;
    // of a crate it vendors, `library/vendor/hashbrown-0.17.1/..`, which
    // Cargo's build of std may take from the registry instead, the same
    // crate, which `bun run std` places as the shipped std names it (#2325).
    let file = match file.find("/registry/src/") {
        Some(at) => match file[at + "/registry/src/".len()..].split_once('/') {
            Some((_, crate_file)) => format!("library/vendor/{crate_file}"),
            None => file.clone(),
        },
        None => file
            .find("/library/")
            .map_or(file.clone(), |at| file[at + 1..].to_string()),
    };
    (file, at.line, at.col_display + 1)
}

/// Each site of `id`'s panics, in the order they're written: each call of
/// a function that tracks its caller, and each check MIR asserts; and those
/// of each function nested in it that it calls, `swap_remove`'s cold
/// `assert_failed`, whose own lines std's hook names.
fn sites(tcx: TyCtxt<'_>, root: DefId) -> Vec<(String, usize, usize)> {
    let mut found: Vec<(String, usize, usize)> = Vec::new();
    let mut each = vec![root];
    let mut seen = IndexSet::default();
    while let Some(id) = each.pop() {
        if !seen.insert(id) {
            continue;
        }
        let body = tcx.optimized_mir(id);
        for block in body.basic_blocks.iter() {
            let terminator = block.terminator();
            match &terminator.kind {
                mir::TerminatorKind::Call {
                    func, fn_span, args, ..
                } => {
                    // A closure of its own it gives what it calls,
                    // `with_borrow`'s `|cell| f(&cell.borrow())` that `with`
                    // calls, or `set`'s, of nothing captured, a constant, to
                    // `initialize_with`.
                    // Or held in another's it's given, an array's clone's
                    // closure in `try_from_fn`'s `wrap_mut_1` (#2471).
                    for arg in args.iter() {
                        let held = (arg.node.ty(body, tcx).walk()).filter_map(|part| part.as_type());
                        for part in held {
                            if let ty::Closure(closure, _) = part.kind()
                                && nested_in(tcx, *closure, root)
                            {
                                each.push(*closure);
                            }
                        }
                    }
                    let Some((callee, _)) = func.const_fn_def() else {
                        continue;
                    };
                    // Or a call of a method whose model raises at its caller,
                    // a derive's `x.clone()` of a `RefCell` (#2471).
                    if tracks_caller(tcx, callee) || crate::recognition::model_raises_at_caller(tcx, callee) {
                        // Codegen places a call by its `fn_span`, a method's
                        // name to its end, `expect(..)`.
                        let info = mir::SourceInfo {
                            span: *fn_span,
                            ..terminator.source_info
                        };
                        found.extend(site(tcx, root, body, info));
                    } else if nested_in(tcx, callee, root) {
                        // A trait's method declared in it, `to_vec_in`'s own
                        // `ConvertVec`, of which each impl in it is followed.
                        match tcx.trait_of_assoc(callee) {
                            Some(trait_) => each.extend(
                                (tcx.all_impls(trait_))
                                    .filter_map(|imp| tcx.impl_item_implementor_ids(imp).get(&callee).copied()),
                            ),
                            None => each.push(callee),
                        }
                    }
                }
                mir::TerminatorKind::Assert { .. } => found.extend(site(tcx, root, body, terminator.source_info)),
                _ => {}
            }
        }
    }
    found.sort_by_key(|site| (site.1, site.2));
    found.dedup();
    found
}

/// Where `root`, an adapter's `step`, `count` or `last`, calls the same step
/// of what it holds, `Iterator::count` of a type it doesn't know: in its
/// own body, a closure or a function nested in it that it gives on, a
/// flatten's `fn count` given to its fold (#2488).
fn held_step_sites(tcx: TyCtxt<'_>, root: DefId, step: &str) -> Vec<(String, usize, usize)> {
    let iterator = std_item(tcx, StdItem::Iterator);
    let mut found: Vec<(String, usize, usize)> = Vec::new();
    let mut each = vec![root];
    let mut seen = IndexSet::default();
    while let Some(id) = each.pop() {
        if !seen.insert(id) {
            continue;
        }
        let body = tcx.optimized_mir(id);
        for block in body.basic_blocks.iter() {
            let terminator = block.terminator();
            let mir::TerminatorKind::Call {
                func, fn_span, args, ..
            } = &terminator.kind
            else {
                continue;
            };
            for arg in args.iter() {
                for part in (arg.node.ty(body, tcx).walk()).filter_map(|part| part.as_type()) {
                    if let ty::Closure(nested, _) | ty::FnDef(nested, _) = *part.kind()
                        && nested_in(tcx, nested, root)
                    {
                        each.push(nested);
                    }
                }
            }
            let Some((callee, _)) = func.const_fn_def() else {
                continue;
            };
            if tcx.trait_of_assoc(callee) == Some(iterator) && tcx.item_name(callee).as_str() == step {
                let info = mir::SourceInfo {
                    span: *fn_span,
                    ..terminator.source_info
                };
                found.extend(site(tcx, root, body, info));
            } else if nested_in(tcx, callee, root) {
                each.push(callee);
            }
        }
    }
    found.sort_by_key(|site| (site.1, site.2));
    found.dedup();
    found
}

/// Whether `inner` is an item nested in `outer`'s body.
fn nested_in(tcx: TyCtxt<'_>, inner: DefId, outer: DefId) -> bool {
    let mut at = inner;
    while let Some(parent) = tcx.opt_parent(at) {
        if parent == outer {
            return true;
        }
        at = parent;
    }
    false
}

/// The sites, `--std-raised`'s: each name's, or each type's of a name,
/// `library/..:line:column`; `Err` naming a function std hasn't, or of no
/// site.
pub(crate) fn raised_catalogue(tcx: TyCtxt<'_>) -> Result<String, String> {
    let mut kept: IndexMap<&str, ::serde_json::Value> = IndexMap::default();
    for &(name, tag, owner, function_name) in RAISED {
        let id =
            function(tcx, owner, function_name).ok_or_else(|| format!("std has no `{function_name}` of {owner:?}"))?;
        // A declared method's, a shim's, is where it's declared.
        let sites = match owner {
            Owner::Declared(..) => vec![located(tcx, tcx.def_span(id))],
            _ => sites(tcx, id),
        };
        if sites.is_empty() {
            return Err(format!("`{function_name}` of {owner:?} raises no panic in its MIR"));
        }
        let sites = ::serde_json::Value::from(
            (sites.into_iter())
                .map(|(file, line, column)| format!("{file}:{line}:{column}"))
                .collect::<Vec<_>>(),
        );
        match tag {
            None => {
                kept.insert(name, sites);
            }
            // Of types whose sites a helper reads by a kind, `signed` or
            // `unsigned`, each the same, as one macro writes them; one that
            // differs is a site no helper names.
            Some(tag) => {
                let typed = kept.entry(name).or_insert_with(|| ::serde_json::json!({}));
                if typed.get(tag).is_some_and(|kept| *kept != sites) {
                    return Err(format!(
                        "`{function_name}` of {owner:?} raises its panic elsewhere than another `{tag}` one's"
                    ));
                }
                typed[tag] = sites;
            }
        }
    }
    held_steps_checked(HELD_STEPS, HELD_STEPS_ELSEWHERE)?;
    let cloned: ::serde_json::Map<String, ::serde_json::Value> = (super::cloned_at::cloned_at(tcx)?.into_iter())
        .map(|(key, sites)| (key, ::serde_json::Value::from(sites)))
        .collect();
    kept.insert("clonedAt", ::serde_json::Value::Object(cloned));
    for &(name, _, owner, step) in HELD_STEPS {
        let id = function(tcx, owner, step).ok_or_else(|| format!("std has no `{step}` of {owner:?}"))?;
        let sites = held_step_sites(tcx, id, step);
        if sites.is_empty() {
            return Err(format!(
                "`{step}` of {owner:?} asks no `{step}` of what it holds in its MIR"
            ));
        }
        let sites = (sites.into_iter()).map(|(file, line, column)| format!("{file}:{line}:{column}"));
        kept.insert(name, ::serde_json::Value::from(sites.collect::<Vec<_>>()));
    }
    Ok(format!(
        "{}\n",
        ::serde_json::to_string_pretty(&kept).expect("the sites serialize")
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `HELD_STEPS` is std's step catalog's both ways: as written, and
    /// refused of a row it lacks or a row the catalog doesn't ask (#2488).
    #[test]
    fn held_steps_are_the_catalogs() {
        assert_eq!(held_steps_checked(HELD_STEPS, HELD_STEPS_ELSEWHERE), Ok(()));
        let lacking: Vec<_> = (HELD_STEPS.iter().copied())
            .filter(|&(row, ..)| row != "skipCount")
            .collect();
        let lacks = held_steps_checked(&lacking, HELD_STEPS_ELSEWHERE).unwrap_err();
        assert!(lacks.contains("`Skip`'s `count`"), "{lacks}");
        let stale_row = (
            "mapCount",
            &["Map"][..],
            Owner::Free(StdCrate::Core, &["iter"]),
            "count",
        );
        let stale: Vec<_> = (HELD_STEPS.iter().copied()).chain([stale_row]).collect();
        let says = held_steps_checked(&stale, HELD_STEPS_ELSEWHERE).unwrap_err();
        assert!(says.contains("`Map`'s `count` asks no `count`"), "{says}");
    }
}
