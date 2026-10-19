//! Read-only library-operation recognition. No function lowering state or JS emission.

mod methods;
pub(super) mod registry;

use super::combinators::{Comb, IterComb, IterSource, StepOp};
use super::fn_def;
use super::format_spec::Radix;
use super::pointers::PointerOp;
use super::representation::Num;
use super::std_types::any::AnyOp;
use super::std_types::clock::ClockOp;
use super::std_types::cow::CowOp;
use super::std_types::hash::{HashOp, Hashed, StdHasher};
use super::std_types::heap::HeapOp;
use super::std_types::io_error::IoErrorOp;
use super::std_types::lazy::LazyOp;
use super::std_types::map::{MapOp, MapOwner, Part, map_method};
use super::std_types::number::{DurationOp, NumOp};
use super::std_types::once::OnceOp;
use super::std_types::option::OptionPlaceOp;
use super::std_types::path::PathOp;
use super::std_types::pin::PinOp;
use super::std_types::process::ProcessOp;
use super::std_types::range::{RangeKind, RangeOp};
use super::std_types::rc::RcOp;
use super::std_types::slice::{SliceOp, SortWith};
use super::std_types::text::{StringEdit, TextOp};
use super::std_types::uninit::UninitOp;
use rustc_ast::Mutability;
use rustc_hir::attrs::lang_items::LangItem;
use rustc_hir::def::DefKind;
use rustc_hir::{self as hir, intravisit};
use rustc_middle::mir::{BinOp, UnOp};
use rustc_middle::thir::{ExprKind, Thir};
use rustc_middle::traits::ImplSource;
use rustc_middle::ty::{self, Ty, TyCtxt};
use rustc_span::def_id::{DefId, LocalDefId};
use rustc_span::hygiene::{ExpnKind, MacroKind};
use rustc_span::{Symbol, sym};

/// Only immutable analysis inputs: recognition cannot record dependencies,
/// allocate names, register helpers, or lower an expression.
#[derive(Clone, Copy)]
pub(super) struct Recognition<'a, 'tcx> {
    pub tcx: TyCtxt<'tcx>,
    pub typing_env: ty::TypingEnv<'tcx>,
    pub trait_impls: &'a [DefId],
    /// What the crate's libraries export (ADR 0100).
    pub foreign: &'a super::library::Foreign<'a, 'tcx>,
}

/// What a channel does (ADR 0142).
#[derive(Clone, Copy, PartialEq)]
pub(super) enum ChannelOp {
    New,
    Send,
    Recv,
    TryRecv,
}

/// A channel's end: the sending one or the receiving one (ADR 0142).
#[derive(Clone, Copy, PartialEq)]
pub(super) enum ChannelEnd {
    Sender,
    Receiver,
}

/// A channel's error (ADR 0142).
#[derive(Clone, Copy, PartialEq)]
pub(super) enum ChannelError {
    /// `RecvError`, of a `recv` of a channel with no senders left.
    Recv,
    /// `TryRecvError`, `Empty` or `Disconnected`.
    TryRecv,
    /// `SendError(item)`, of a `send` to a channel with no receiver.
    Send,
}

/// The std functions whose JS meaning rust-js knows (ADRs 0023, 0025).
#[derive(Clone, Copy, PartialEq)]
pub(super) enum Std {
    /// A `Vec`'s capacity's (ADR 0419).
    Capacity(CapacityOp),
    /// The allocator's, or a `Layout`'s (ADR 0417).
    Alloc(super::std_types::alloc::AllocOp),
    /// `Option::from(x)`, or `x.into()` of one: `Some(x)`.
    OptionFrom,
    /// The argument itself: `Box::new(x)`, `Rc::new(x)`,
    /// `s.to_owned()`, `String::from(s)`, `v.iter()`, and `Deref` of
    /// `String`, `Rc`, `Vec`, `Ref`, `RefMut` and JS objects.
    Same,
    /// `s.leak()` of a `String`, `Box::leak(b)`: kept for good, which in JS,
    /// freeing nothing itself, is the value, its `&mut` a box of it where
    /// it's one JS can't change in place, `{ value: s }` (ADR 0334).
    Leak,
    /// `o.as_ref()` and `o.as_mut()` of an `Option`: the `Option` its
    /// reference points at, whose items' references are the items (ADR 0023),
    /// and whose `&mut`s to what isn't an object a std call's items (ADR 0099).
    Pointee,
    /// `u64::from(x)` of a number that isn't a BigInt: `BigInt(x)` (ADR 0086).
    ToBig,
    /// `usize::from(flag)` or `u32::from(c)`, `flag.into()` (`into`): a
    /// number from a `bool` or a `char`, as `as` makes it (ADR 0358).
    Cast {
        into: bool,
    },
    /// `u8::try_from(x)` (`into: false`) or `x.try_into()` between
    /// integers: `Ok` of it in range, or a `TryFromIntError`.
    TryFromInt {
        into: bool,
    },
    /// An iterator's `cloned()` and `copied()`: its items, each cloned if
    /// that could be told apart from sharing it (ADR 0052).
    Cloned,
    /// `v[i]` of a `Vec`, `Index::index` or `IndexMut::index_mut`: `$index(v, i)`.
    Index,
    /// A channel's: `mpsc::channel()`, `send`, `recv` and `try_recv` (ADR 0142).
    Channel(ChannelOp),
    /// `Cell::new(x)` and `RefCell::new(x)`: `{ value: x }`.
    CellNew,
    CellGet,
    CellSet,
    /// A `Cell`'s or `RefCell`'s `replace(v)`, or `take()`: what it held.
    CellReplace,
    CellTake,
    /// `RefCell::replace_with(f)`.
    CellReplaceWith,
    /// A `Cell`'s or `RefCell`'s `get_mut()`, `swap(&other)`, and a `Cell`'s
    /// `update(f)` (ADR 0327).
    CellGetMut,
    CellSwap,
    CellUpdate,
    /// `Cell::from_mut(r)`, a `&mut` as a `&Cell` (ADR 0153).
    CellFromMut,
    /// `as_slice_of_cells()` and `as_array_of_cells()`: a cell of each item.
    CellItems,
    /// A lock's `is_poisoned()`: `false`, as nothing can catch a panic that
    /// holds it (ADR 0327).
    NotPoisoned,
    /// A `PoisonError`'s `into_inner()` and `get_ref()`: what it holds, its
    /// guard, which a lock never poisoned gives in its `Ok` (ADR 0449).
    PoisonData,
    /// `RefCell::borrow`, `borrow_mut`, a `Mutex`'s `lock()` and an `RwLock`'s
    /// `read()` and `write()`, `Ok` of it: a guard, which is the cell, checked
    /// free to borrow so (ADR 0328).
    Borrow {
        mutable: bool,
        lock: bool,
    },
    /// `try_borrow()`, `try_borrow_mut()`: `Ok` of a guard, or `Err` (ADR 0328);
    /// a lock's `try_lock()`, `try_read()` and `try_write()` (`lock`).
    TryBorrow {
        mutable: bool,
        lock: bool,
    },
    /// What a guard guards: its cell's `value`, and a `&mut` to a number or
    /// text in it the cell, the `{ value }` box that is (ADR 0328).
    GuardValue {
        mutable: bool,
    },
    /// A lock's `into_inner()` and `get_mut()`: `Ok` of its `value`.
    Lock,
    /// A `Pin`'s functions, each its pointer's (ADR 0329).
    Pin(PinOp),
    /// `TypeId`s and a `dyn Any`'s downcasts (ADR 0331).
    Any(AnyOp),
    /// The process a program runs in, Node's (ADR 0391): `env::args()`.
    Process(ProcessOp),
    /// `panic::Location`'s (ADR 0427).
    Location(super::std_types::location::LocationOp),
    /// An `io::Error`'s: made, or its kind asked (ADR 0462).
    IoError(IoErrorOp),
    /// Hashing's (ADR 0463): a std hasher made, or its own method called, or
    /// a std type's `Hash`.
    Hash(HashOp),
    /// A path's own method (ADR 0405).
    Path(PathOp),
    /// std's of a raw pointer, a handle on its place (ADR 0407).
    Pointer(PointerOp),
    /// `MaybeUninit`'s, and a `Box`'s made before its value (ADR 0332).
    Uninit(UninitOp),
    /// An atomic's operations (ADR 0096), on its `{ value }` as a `Cell`'s:
    /// `load` and `into_inner`, `store`, `swap`, the `fetch_` ones, with
    /// the operator or whether it's `fetch_max`, and `compare_exchange`.
    AtomicLoad,
    AtomicStore,
    AtomicSwap,
    AtomicFetch(BinOp),
    AtomicFetchMax(bool),
    AtomicCompareExchange,
    ToString,
    /// `String + &str`.
    Concat,
    StringNew,
    /// `trim()`, `trim_start()` or `trim_end()`, by Unicode's White_Space.
    Trim {
        start: bool,
        end: bool,
    },
    /// `is_empty` on a string or a `Vec`: `x.length === 0`.
    IsEmpty,
    VecNew,
    /// What JS has nothing of: a `Vec`'s capacity asked for, `reserve`,
    /// `shrink_to_fit`, and `thread::yield_now()`, of the one thread there
    /// is (ADR 0142): its arguments, for what they do, and `undefined`
    /// (ADR 0315).
    Nothing,
    /// `vec![a, b]`.
    VecMacro,
    Push,
    Len,
    /// `len()` of an iterator of the crate's whose `ExactSizeIterator` keeps
    /// std's `len`: its `size_hint()`, checked (ADR 0164).
    ExactLen,
    /// A `Duration`'s own, of its nanoseconds (ADR 0188).
    Duration(DurationOp),
    /// A clock's, `SystemTime`'s or, `instant`, `Instant`'s, of its
    /// nanoseconds (ADR 0447).
    Clock {
        op: ClockOp,
        instant: bool,
    },
    /// `panic!("{}", x)`: a panic of `x`'s `Display` (ADR 0189); and an
    /// edition 2018 `unreachable!(x)`'s, `unreachable_display(&x)`, after
    /// std's words, as edition 2021's are.
    PanicDisplay {
        unreachable: bool,
    },
    /// `<&[T; N]>::try_from(slice)`, or `<[T; N]>`'s: `Ok` of the slice where
    /// it's `N` long (ADR 0189).
    SliceToArray {
        into: bool,
    },
    /// `format!(..)`'s `format`: its `format_args!`, a string already (ADR
    /// 0034), or std's panic where what it formats fails (ADR 0187).
    Format,
    /// `it.by_ref()`: the iterator itself, which knows where it is.
    IterByRef,
    /// `NonZero::new(n)`: `None` of `0`, else `n` (ADR 0177).
    NonZeroNew,
    /// `it.size_hint()`: std's `(0, None)` of an iterator of the crate's that
    /// keeps it, or, `true`, `(n, Some(n))` of std's that knows its length
    /// (ADR 0170).
    SizeHint(bool),
    /// An operator of `std::num::Wrapping`, its number's in its `[x]`
    /// (ADR 0175): `a * b`, `-a`, or, `true`, `a += b`.
    WrappingOp(Result<BinOp, UnOp>, bool),
    /// `write!(w, ..)` or `w.write_char(c)` of a writer of the crate's that
    /// keeps std's: its own `write_str`, given the text whole (ADR 0166).
    UserWrite,
    /// `write!(w, ..)` or `w.write_all(bytes)` of a writer of bytes of the
    /// crate's that keeps std's: its dictionary's, made of its `write`
    /// (ADR 0392).
    UserIoWrite,
    Clear,
    Retain,
    /// `panic!("..")`, `assert!(..)`: `throw new Error(..)`.
    Panic,
    /// `panic!("{}", x)`: the same, with a formatted message.
    PanicFmt,
    /// `panic!(x)` before edition 2021, `std::rt::begin_panic(x)`: the
    /// same, with `x` as it is, if it's text.
    BeginPanic,
    /// `hint::unreachable_unchecked()`: MIR's unreachable code's throw.
    Unreachable,
    /// `size_of::<T>()` and `align_of::<T>()`, and `size_of_val(&x)` and
    /// `align_of_val(&x)` of a sized `x`: the wasm32 target's, as their
    /// constants are (ADR 0090).
    Layout {
        fact: TypeFact,
        of_val: bool,
    },
    /// `mem::needs_drop::<T>()`: rustc's answer, `true` or `false`, or a
    /// type parameter's, given (ADR 0145).
    NeedsDrop,
    /// `mem::discriminant(&e)`: `e`'s variant's discriminant (ADR 0393).
    Discriminant,
    /// `mem::transmute::<A, B>(a)` of plain data (ADR 0408): its parts as
    /// `B`'s.
    Transmute,
    /// `mem::drop(x)`: `x`'s destructor, and `mem::forget(x)`: none (ADR 0098).
    Drop,
    Forget,
    /// `mem::swap(&mut a, &mut b)` and `mem::replace(&mut a, v)` of places:
    /// each written in turn.
    Swap,
    Replace,
    /// `dest.clone_from(&source)`: `mem::replace(dest, source.clone())`'s
    /// write, of a clone of the source.
    CloneFrom,
    /// `opt.take()`, `opt.replace(v)` and `mem::take(&mut x)`: `mem::replace`
    /// with `None`, `Some(v)` and `x`'s default (ADR 0136).
    OptionTake,
    OptionReplace,
    MemTake,
    /// `opt.flatten()`: the inner `Option`, `$someValue(opt)` (ADR 0051).
    OptionFlatten,
    /// `s.to_ascii_lowercase()`: only ASCII's letters, `$asciiCase(s)`,
    /// unlike JS's `toLowerCase` (ADR 0136).
    AsciiCase {
        upper: bool,
    },
    /// `a.eq_ignore_ascii_case(b)`.
    AsciiEq,
    /// `v.append(&mut other)`: `$append(v, other)`, which empties `other`.
    Append,
    /// `write!(v, ..)` of a `Vec<u8>`, an `io::Write`: its text's UTF-8
    /// bytes added, `$writeText(v, text)` (ADR 0392).
    WriteText,
    /// `println!` and `print!`, or with `error`, `eprintln!` and
    /// `eprint!`: `console.log(..)` of a line (ADR 0087).
    Print {
        error: bool,
    },
    /// What `assert_eq!` and `assert_ne!` call when they fail.
    AssertFailed,
    /// `format_args!("..")` with no placeholders: the string.
    FmtStr,
    /// `format_args!("{} {:?}", ..)`: a template and its arguments.
    FmtNew,
    /// An argument for `{}`.
    FmtDisplay,
    /// An argument for `{:?}`.
    FmtDebug,
    /// `Argument::new_lower_hex` and the like: `{:x}` (ADR 0058).
    FmtRadix(Radix),
    /// `{:e}` (false) and `{:E}`: exponent notation.
    FmtExp(bool),
    /// `{:p}`: of a type of the crate's, its own `Pointer` (ADR 0165).
    FmtPointer,
    /// `Argument::from_usize`: a width or precision from an argument, `{:>w$}`.
    FmtUsize,
    /// A `HashMap` or `HashSet` method (ADR 0059).
    Map(MapOp),
    /// A `char` or `str` method, `parse`, or slicing by a range (ADR 0063).
    Text(TextOp),
    /// A `String` changed in place (ADR 0149).
    StringEdit(StringEdit),
    /// `String::with_capacity(n)`: an empty string.
    StringWithCapacity,
    /// A range's method, or `a..=b` (ADR 0129).
    Range(RangeOp),
    /// `any::type_name::<T>()`, and `type_name_of_val(&x)`: rustc's name for
    /// `T`, a string (ADR 0132).
    TypeName {
        of_val: bool,
    },
    /// Standard output and error (ADR 0132): `io::stdout()` and its `lock()`,
    /// which hold nothing JS needs, writes to one, and an `io::Result<()>`'s
    /// `unwrap()`, as a write's is always `Ok`.
    Stream(StreamOp),
    Number(NumOp),
    /// A `BinaryHeap`'s own methods (ADR 0068).
    Heap(HeapOp),
    /// A `OnceCell`'s or `OnceLock`'s (ADR 0317).
    Once(OnceOp),
    /// A `Cow`'s (ADR 0319).
    Cow(CowOp),
    /// An `Option`'s methods that write its place (ADR 0326).
    OptionPlace(OptionPlaceOp),
    /// A counted `Rc`'s or a `Weak`'s (ADR 0320).
    Rc(RcOp),
    /// A slice's fills, copies, order checks, chunks and splits (ADR 0324).
    Slice(SliceOp),
    /// A `LazyCell`'s or `LazyLock`'s (ADR 0318).
    Lazy(LazyOp),
    /// `serde_json::to_string(&v)` (false) and `to_string_pretty` (ADR 0077).
    ToJson(bool),
    /// `serde_json::from_str::<T>(s)` (ADR 0078).
    FromJson,
    /// `it.next()`, `peekable()`, `peek()` and the like (ADR 0071).
    Step(StepOp),
    /// `VecDeque::remove(i)`: an `Option`, where `Vec`'s panics.
    DequeRemove,
    /// `vec![x; n]`.
    FromElem,
    /// An `Option`, `Result` or `Vec` method (ADR 0062).
    Comb(Comb),
    /// An iterator adapter or consumer (ADR 0062).
    IterComb(IterComb),
    /// `std::iter::once(x)` and std's other iterator sources (ADR 0128).
    IterSource(IterSource),
    /// An `Option`'s `iter()` and `into_iter()`: an array of its value, or
    /// of none (ADR 0128).
    OptionIter,
    /// `iter_mut()` of an `Option`, and `&mut option` iterated: a handle
    /// on its own place, of a number or text (ADR 0343).
    OptionIterMut,
    /// `Option` (ADR 0030): `o != null`, `o == null`.
    IsSome,
    IsNone,
    /// `unwrap()` and `expect(msg)`: `$unwrap(o)`, `$unwrap(o, msg)`.
    Unwrap,
    /// `unwrap_unchecked()`: the value, unchecked, TypeScript's `o!` (ADR 0294).
    UnwrapUnchecked,
    /// `unwrap_or(d)`: `o ?? d`.
    UnwrapOr,
    /// `a += b` of numbers where `b` is a reference: `a = a + b`.
    AssignOperator(BinOp),
    /// `copied()` and `cloned()` of an `Option<&T>`: a clone of what's in it.
    OptionCloned,
    /// `map(f)`: `o != null ? f(o) : undefined`, with a closure's body in place.
    OptionMap,
    /// A string method that is a JS one (ADR 0034): `s.starts_with(p)` is
    /// `s.startsWith(p)`. Also `join` on a slice of strings.
    Method(&'static str),
    /// `strip_prefix` and `strip_suffix`: an option (ADR 0030).
    StripPrefix,
    StripSuffix,
    /// `strip_circumfix(p, s)`: `strip_prefix`'s, then `strip_suffix`'s of
    /// what's left (ADR 0334).
    StripCircumfix,
    /// `split_once` and `rsplit_once`: an option of the two sides.
    SplitOnce,
    RsplitOnce,
    /// `s.push_str(t)` and `s.push(c)`: `s = s + t`.
    PushStr,
    /// `.last()` of a `split`: `.at(-1)`.
    Last,
    /// A slice's `first()` and `last()`: `v[0]` and `v.at(-1)`.
    First,
    SliceLast,
    /// `v.get(i)`: `v[i]`, which is `undefined` past the end; and
    /// `get_unchecked(i)`, which Rust leaves undefined there.
    SliceGet,
    /// `char::from_digit(n, radix)` and `char::from_u32(n)`: `None` when there's no such `char`.
    FromDigit,
    FromU32,
    /// `Result` (ADR 0035): `r.TAG === "Ok"` (true) or `"Err"` (false).
    IsOk(bool),
    /// `r.ok()`: the value, or `undefined`.
    ResultOk,
    /// `r.unwrap()`, `r.expect(msg)`: `$unwrapOk(r)`.
    UnwrapOk,
    /// `r.unwrap_err()`, `r.expect_err(msg)`: `$unwrapErr(r)`.
    UnwrapErr,
    /// `r.unwrap_or(d)`.
    ResultOr,
    /// An iterator's adapter or consumer that is the array's method (ADR 0036):
    /// `map`, `filter`, `any` (`some`), `all` (`every`), `find`, `for_each`.
    ArrayMethod(&'static str),
    Enumerate,
    Rev,
    Skip,
    Take,
    Fold,
    Sum,
    CollectString,
    /// `collect::<Vec<_>>()`: a new array, unless it's one already.
    Collect,
    Position,
    /// `max()` (true) or `min()` (false) of an iterator: an option.
    Extreme(bool),
    Chars,
    ToVec,
    /// `join` of a slice of lists, not of text: `$joinWith` (ADR 0322).
    JoinItems,
    Sort,
    SortBy,
    SortByKey,
    /// `a.cmp(&b)`: -1, 0 or 1 (ADR 0036).
    Cmp,
    /// `a.max(b)` (true) or `a.min(b)` (false) of two numbers.
    MaxOf(bool),
    /// `std::ptr::eq(a, b)`: of JS objects, whether they're one (ADR 0285).
    PtrEq,
    /// An operator on references to numbers, `x % 10` with `x: &i32`,
    /// which rustc writes as a call of the operator's trait.
    Operator(BinOp),
    /// `-x` or `!b` of a reference to a number or a `bool`, likewise.
    UnaryOperator(UnOp),
    /// A thread-local's `with(f)`: `f(key)`; and `with_borrow(f)`,
    /// `with_borrow_mut(f)` of a `RefCell` one: `f` of its value, while it's
    /// borrowed (ADR 0328).
    LocalWith,
    LocalBorrow {
        mutable: bool,
    },
    /// `Ordering::then`, `then_with`, `reverse`.
    Then,
    ThenWith,
    Reverse,
}

/// A `Vec`'s capacity's (ADR 0419).
#[derive(Clone, Copy, PartialEq, Debug)]
pub(crate) enum CapacityOp {
    /// `Vec::with_capacity(n)`.
    WithCapacity,
    /// `v.capacity()`.
    Of,
    /// `v.reserve(n)`, `reserve_exact(n)`.
    Reserve,
    /// `v.shrink_to(n)`, or `shrink_to_fit()`.
    ShrinkTo { fit: bool },
}

/// std's collections that are a JS array, whose capacity is the array's
/// (ADRs 0068, 0419): the paths of their methods, of `T` and an allocator.
const ARRAY_COLLECTIONS: [&str; 3] = [
    "std::vec::Vec::<T, A>::",
    "std::collections::VecDeque::<T, A>::",
    "std::collections::BinaryHeap::<T, A>::",
];

/// Which of a `Vec`'s capacity's functions `def_id` is, or a `VecDeque`'s
/// or a `BinaryHeap`'s, one of `ARRAY_COLLECTIONS`.
pub(crate) fn capacity_op(tcx: TyCtxt<'_>, def_id: DefId) -> Option<CapacityOp> {
    let path = std_path(tcx, def_id);
    if matches!(
        path.as_str(),
        "std::vec::Vec::<T>::with_capacity"
            | "std::collections::VecDeque::<T>::with_capacity"
            | "std::collections::BinaryHeap::<T>::with_capacity"
    ) {
        return Some(CapacityOp::WithCapacity);
    }
    Some(match array_collection_method(&path)? {
        "capacity" => CapacityOp::Of,
        "reserve" | "reserve_exact" => CapacityOp::Reserve,
        "shrink_to_fit" => CapacityOp::ShrinkTo { fit: true },
        "shrink_to" => CapacityOp::ShrinkTo { fit: false },
        _ => return None,
    })
}

/// Whether `def_id`, of one of `ARRAY_COLLECTIONS`, may shorten it, which
/// keeps its capacity (ADR 0419): the array it's given first, or its
/// second, `append`'s.
pub(crate) fn shortens_vec(tcx: TyCtxt<'_>, def_id: DefId) -> Option<usize> {
    match array_collection_method(&std_path(tcx, def_id))? {
        "append" => Some(1),
        "clear" | "truncate" | "set_len" | "pop" | "pop_if" | "pop_front" | "pop_back" | "pop_front_if"
        | "pop_back_if" | "remove" | "swap_remove" | "drain" | "retain" | "retain_mut" | "dedup" | "dedup_by"
        | "dedup_by_key" | "split_off" | "resize" | "resize_with" | "extract_if" | "splice" => Some(0),
        _ => None,
    }
}

/// The method of one of `ARRAY_COLLECTIONS` that `path` names.
fn array_collection_method(path: &str) -> Option<&str> {
    ARRAY_COLLECTIONS.iter().find_map(|prefix| path.strip_prefix(prefix))
}

/// What a call does with a standard stream (ADR 0132).
#[derive(Clone, Copy, PartialEq, Debug)]
pub(super) enum StreamOp {
    /// `io::stdout()` and `io::stderr()`: `undefined`.
    Open,
    /// `stdout.lock()`, `out.flush()` and `write.unwrap()`: what it's called
    /// on, run for its effects, and `undefined`.
    Nothing,
    /// `write!(out, ..)`: `print!` or `eprint!`.
    Write { error: bool },
    /// `out.write_all(bytes)`: those bytes written, `$printBytes(bytes,
    /// error)` (ADR 0392).
    WriteBytes { error: bool },
    /// `out.write(bytes)`: those bytes written, and `Ok` of how many,
    /// which is all of them (ADR 0392).
    Written { error: bool },
}

impl Std {
    /// Does it give its `&mut` to a number or text as a cell already: its
    /// receiver, the `{ value }` box that is (ADRs 0317, 0318, 0320), or one it
    /// makes (ADR 0327): no handle (ADR 0152) to make.
    pub(super) fn gives_its_cell(self) -> bool {
        matches!(
            self,
            Std::Once(OnceOp::GetMut)
                | Std::Leak
                | Std::OptionIterMut
                | Std::Map(
                    MapOp::EntryGet { mutable: true } | MapOp::VacantInsert | MapOp::TreeRange { mutable: true }
                )
                | Std::Slice(SliceOp::DequeRangeMut)
                // A generic index's `index_mut`, its dictionary's handle (ADR 0418).
                | Std::Slice(SliceOp::GenericIndex { mutable: true })
                | Std::Comb(Comb::ResultIterMut | Comb::ResultAsMut)
                | Std::Lazy(LazyOp::GetMut | LazyOp::ForceMut)
                | Std::Rc(RcOp::GetMut | RcOp::MakeMut)
                | Std::CellGetMut
                | Std::GuardValue { mutable: true }
                | Std::Pin(PinOp::Mut | PinOp::Map)
                // A pointer's `as_mut()`, its handle, a cell (ADR 0407), and
                // a `NonNull`'s.
                | Std::Pointer(PointerOp::AsMut | PointerOp::PlaceMut)
                | Std::Heap(HeapOp::PeekTop { mutable: true })
                | Std::Slice(
                    SliceOp::PushMut { .. }
                        | SliceOp::PushFrontMut
                        | SliceOp::DequeInsert { mutable: true }
                        | SliceOp::SplitEndMut { .. }
                        | SliceOp::SplitOff { mutable: true, .. }
                        | SliceOp::GetDisjointMut { .. }
                        | SliceOp::SelectNth { .. }
                )
                | Std::Any(AnyOp::DowncastMut)
                | Std::Uninit(UninitOp::Write | UninitOp::InitMut)
                | Std::Text(
                    TextOp::EncodeUtf8
                        | TextOp::StrPart { .. }
                        | TextOp::StrGetMut
                        | TextOp::StrSplitAtMut { .. }
                        | TextOp::FromUtf8Mut { .. }
                )
                | Std::OptionPlace(
                    OptionPlaceOp::GetOrInsert
                        | OptionPlaceOp::GetOrInsertWith
                        | OptionPlaceOp::GetOrInsertDefault
                        | OptionPlaceOp::Insert
                )
        )
    }
}

/// serde_json's own types, by name.
#[derive(Clone, Copy, PartialEq)]
pub(super) enum Json {
    Value,
    Number,
    Map,
}

impl<'a, 'tcx> Recognition<'a, 'tcx> {
    /// std's `FromStr`, found among the traits, as it has no diagnostic item.
    pub(super) fn std_from_str(&self) -> Option<DefId> {
        self.tcx
            .all_traits_including_private()
            .find(|&id| is_from_str(self.tcx, id))
    }

    pub(super) fn classify(&self, def_id: DefId, args: ty::GenericArgsRef<'tcx>) -> Option<Std> {
        if let Some(decided) = self.classify_fn(def_id, args) {
            return decided;
        }
        if let Some(trait_) = self.tcx.trait_of_assoc(def_id) {
            return self.classify_trait_method(def_id, trait_, args);
        }
        self.classify_inherent(def_id, args)
    }

    /// A free function's, or one rust-js knows by its identity, as `mem::swap`
    /// and `Box::new`: `Some` once decided, of what it is or that it's none
    /// of std's, and `None` to look further, as a method.
    fn classify_fn(&self, def_id: DefId, args: ty::GenericArgsRef<'tcx>) -> Option<Option<Std>> {
        let tcx = self.tcx;
        let diagnostic = |name: &str| tcx.is_diagnostic_item(Symbol::intern(name), def_id);
        let self_ty = args.types().next();
        if let Some((fact, of_val)) = layout_query(tcx, def_id) {
            return Some(Some(Std::Layout { fact, of_val }));
        }
        // A write to a `Formatter` held where it isn't its writer's, a `&mut`
        // to it kept in a struct: its text's, a `String`'s, through the
        // handle it is (ADR 0461).
        if self.formatter_write(def_id, args) {
            return Some(Some(Std::PushStr));
        }
        if type_facts(tcx, def_id) == [TypeFact::NeedsDrop] {
            return Some(Some(Std::NeedsDrop));
        }
        // An enum's variant's discriminant (ADR 0393).
        // And as a derived `Hash` hashes it (ADR 0463).
        if matches!(
            std_path(tcx, def_id).as_str(),
            "std::mem::discriminant" | "std::intrinsics::discriminant_value"
        ) {
            return Some(Some(Std::Discriminant));
        }
        if diagnostic("mem_drop") {
            return Some(Some(Std::Drop));
        }
        if diagnostic("mem_forget") {
            return Some(Some(Std::Forget));
        }
        // The one thread there is yields to none (ADR 0142).
        if std_path(tcx, def_id) == "std::thread::yield_now" {
            return Some(Some(Std::Nothing));
        }
        // `hint::assert_unchecked(c)`, a promise that `c` holds, which a
        // program that's right keeps: nothing, its `c` made as it's given
        // (ADR 0399).
        if std_path(tcx, def_id) == "std::hint::assert_unchecked" {
            return Some(Some(Std::Nothing));
        }
        // Plain data's bytes as another type's, and all zero (ADR 0408).
        match std_path(tcx, def_id).as_str() {
            "std::intrinsics::transmute" | "std::intrinsics::transmute_unchecked" => {
                return Some(Some(Std::Transmute));
            }
            "std::mem::zeroed" => return Some(Some(Std::Uninit(UninitOp::Zeroed))),
            "std::array::from_fn" => return Some(Some(Std::Slice(SliceOp::FromFn))),
            _ => {}
        }
        // A `NonNull` of a reference, `NonNull::from(&mut x)`: a pointer to
        // what it refers to (ADR 0407).
        if diagnostic("from_fn")
            && self_ty
                .is_some_and(|t| matches!(t.kind(), ty::Adt(adt, _) if is_std_def(tcx, adt.did(), StdItem::NonNull)))
            && args.types().nth(1).is_some_and(|t| t.is_ref())
        {
            return Some(Some(Std::Pointer(super::pointers::PointerOp::FromRef)));
        }
        // A raw pointer's, a handle on its place (ADR 0407).
        if let Some(op) = super::pointers::pointer_op(&std_path(tcx, def_id)) {
            return Some(Some(Std::Pointer(op)));
        }
        // A `Vec`'s capacity's, kept where it's asked (ADR 0419).
        if let Some(op) = capacity_op(tcx, def_id) {
            return Some(Some(Std::Capacity(op)));
        }
        // The allocator's, an array of slots (ADR 0417).
        if let Some(op) = super::std_types::alloc::alloc_op(&std_path(tcx, def_id)) {
            return Some(Some(Std::Alloc(op)));
        }
        // The process a program runs in, Node's (ADR 0391).
        if let Some(op) = node_only(tcx, def_id) {
            return Some(Some(Std::Process(op)));
        }
        // A C-variadic function's next argument, the first of what's left of
        // its array (ADR 0428).
        if std_path(tcx, def_id) == "std::ffi::VaList::<'f>::next_arg" {
            return Some(Some(Std::Method("shift")));
        }
        // Where a call is, `Location::caller()`, and its parts (ADR 0427).
        if let Some(op) = location_of(tcx, def_id) {
            return Some(Some(Std::Location(op)));
        }
        if let Some(op) = self.io_error_op(def_id, args) {
            return Some(Some(Std::IoError(op)));
        }
        if let Some(op) = self.hash_op(def_id, args) {
            return Some(Some(Std::Hash(op)));
        }
        // Bytes to text, as Rust validates UTF-8 (ADR 0172).
        let utf8 = match std_path(tcx, def_id).as_str() {
            "std::str::from_utf8" | "std::str::<impl str>::from_utf8" => Some(TextOp::FromUtf8 { owned: false }),
            "std::string::String::from_utf8" => Some(TextOp::FromUtf8 { owned: true }),
            "std::str::from_utf8_unchecked"
            | "std::str::<impl str>::from_utf8_unchecked"
            | "std::string::String::from_utf8_unchecked" => Some(TextOp::Utf8Unchecked),
            "std::string::String::from_utf8_lossy" => Some(TextOp::Utf8Lossy),
            "std::str::from_utf8_mut" | "std::str::<impl str>::from_utf8_mut" => {
                Some(TextOp::FromUtf8Mut { unchecked: false })
            }
            "std::str::from_utf8_unchecked_mut" | "std::str::<impl str>::from_utf8_unchecked_mut" => {
                Some(TextOp::FromUtf8Mut { unchecked: true })
            }
            "std::str::Utf8Error::valid_up_to" => Some(TextOp::Utf8Part("valid_up_to")),
            // A `Utf8Chunk`'s parts, `{ valid, invalid }` (ADR 0341).
            "std::str::Utf8Chunk::<'a>::valid" => Some(TextOp::Utf8Part("valid")),
            "std::str::Utf8Chunk::<'a>::invalid" => Some(TextOp::Utf8Part("invalid")),
            "std::num::ParseIntError::kind" => Some(TextOp::ParseErrorKind),
            _ => None,
        };
        if let Some(op) = utf8 {
            return Some(Some(Std::Text(op)));
        }
        // A path is its text (ADR 0173): what reads it as text is itself.
        match std_path(tcx, def_id).as_str() {
            "std::path::Path::new"
            | "std::path::Path::display"
            | "std::path::Path::to_str"
            | "std::path::Path::as_os_str"
            | "std::ffi::OsStr::to_str"
            | "std::path::PathBuf::as_path" => return Some(Some(Std::Same)),
            "std::path::PathBuf::new" => return Some(Some(Std::StringNew)),
            // Its methods, over its components (ADR 0405).
            "std::path::Path::join" => return Some(Some(Std::Path(PathOp::Join))),
            "std::path::Path::parent" => return Some(Some(Std::Path(PathOp::Parent))),
            "std::path::Path::file_name" => return Some(Some(Std::Path(PathOp::FileName))),
            "std::path::Path::file_stem" => return Some(Some(Std::Path(PathOp::FileStem))),
            "std::path::Path::extension" => return Some(Some(Std::Path(PathOp::Extension))),
            "std::path::Path::with_extension" => return Some(Some(Std::Path(PathOp::WithExtension))),
            "std::path::Path::with_file_name" => return Some(Some(Std::Path(PathOp::WithFileName))),
            "std::path::Path::starts_with" => return Some(Some(Std::Path(PathOp::StartsWith))),
            "std::path::Path::ends_with" => return Some(Some(Std::Path(PathOp::EndsWith))),
            "std::path::Path::strip_prefix" => return Some(Some(Std::Path(PathOp::StripPrefix))),
            "std::path::Path::is_absolute" | "std::path::Path::has_root" => {
                return Some(Some(Std::Path(PathOp::IsAbsolute)));
            }
            "std::path::Path::is_relative" => return Some(Some(Std::Path(PathOp::IsRelative))),
            "std::path::Path::iter" => return Some(Some(Std::Path(PathOp::Iter))),
            "std::path::Path::ancestors" => return Some(Some(Std::Path(PathOp::Ancestors))),
            "std::path::Path::to_path_buf" => return Some(Some(Std::Same)),
            "std::path::PathBuf::push" => return Some(Some(Std::StringEdit(StringEdit::PathPush))),
            "std::path::PathBuf::pop" => return Some(Some(Std::StringEdit(StringEdit::PathPop))),
            "std::path::PathBuf::set_extension" => return Some(Some(Std::StringEdit(StringEdit::SetExtension))),
            "std::path::PathBuf::set_file_name" => return Some(Some(Std::StringEdit(StringEdit::SetFileName))),
            // A `ManuallyDrop` is what it holds (ADR 0406), never dropped.
            "std::mem::ManuallyDrop::<T>::new" | "std::mem::ManuallyDrop::<T>::into_inner" => {
                return Some(Some(Std::Same));
            }
            // A `NonZero` is its number (ADR 0177).
            "std::num::NonZero::<T>::get" | "std::num::NonZero::<T>::new_unchecked" => return Some(Some(Std::Same)),
            "std::num::NonZero::<T>::new" => return Some(Some(Std::NonZeroNew)),
            _ => {}
        }
        let utf8 = match std_path(tcx, def_id).as_str() {
            "std::str::Utf8Error::error_len" => Some(TextOp::Utf8Part("error_len")),
            "std::string::FromUtf8Error::utf8_error" => Some(TextOp::Utf8Part("error")),
            "std::string::FromUtf8Error::into_bytes" | "std::string::FromUtf8Error::as_bytes" => {
                Some(TextOp::Utf8Part("bytes"))
            }
            _ => None,
        };
        if let Some(op) = utf8 {
            return Some(Some(Std::Text(op)));
        }
        if std_path(tcx, def_id) == "std::sync::mpsc::channel" {
            return Some(Some(Std::Channel(ChannelOp::New)));
        }
        if diagnostic("mem_swap") {
            return Some(Some(Std::Swap));
        }
        if diagnostic("ptr_eq") {
            return Some(Some(Std::PtrEq));
        }
        if diagnostic("mem_replace") {
            return Some(Some(Std::Replace));
        }
        // `Clone::clone_from`, std's own: `*dest = source.clone()`. A crate's
        // impl of its own is its own.
        if tcx
            .trait_of_assoc(def_id)
            .is_some_and(|t| tcx.is_lang_item(t, LangItem::Clone))
            && tcx.item_name(def_id).as_str() == "clone_from"
        {
            return Some(Some(Std::CloneFrom));
        }
        if tcx.crate_name(def_id.krate) == sym::core && std_path(tcx, def_id) == "std::mem::take" {
            return Some(Some(Std::MemTake));
        }
        // `cmp::max(a, b)` of numbers is `a.max(b)`'s (ADR 0136); of anything
        // else, the call is lowered as `Ord::max`'s.
        if let Some(max) = cmp_extreme(tcx, def_id)
            && self_ty.and_then(Num::of).is_some_and(|num| !num.float())
        {
            return Some(Some(Std::MaxOf(max)));
        }
        if tcx.is_lang_item(def_id, LangItem::RangeInclusiveNew) {
            return Some(Some(Std::Range(RangeOp::New)));
        }
        if tcx.crate_name(def_id.krate) == sym::std {
            match std_path(tcx, def_id).as_str() {
                "std::io::stdout" | "std::io::stderr" => return Some(Some(Std::Stream(StreamOp::Open))),
                _ => {}
            }
        }
        if tcx.crate_name(def_id.krate) == sym::core {
            match std_path(tcx, def_id).as_str() {
                "std::any::type_name" => return Some(Some(Std::TypeName { of_val: false })),
                "std::any::type_name_of_val" => return Some(Some(Std::TypeName { of_val: true })),
                _ => {}
            }
        }
        // std's iterator sources (ADR 0128), by path: most have no
        // diagnostic item.
        if tcx.crate_name(def_id.krate) == sym::core && tcx.def_kind(def_id) == DefKind::Fn {
            let source = match std_path(tcx, def_id).as_str() {
                "std::iter::once" => Some(IterSource::Once),
                "std::iter::empty" => Some(IterSource::Empty),
                "std::iter::repeat" => Some(IterSource::Repeat),
                "std::iter::repeat_with" => Some(IterSource::RepeatWith),
                "std::iter::successors" => Some(IterSource::Successors),
                "std::iter::from_fn" => Some(IterSource::FromFn),
                _ => None,
            };
            if let Some(source) = source {
                return Some(Some(Std::IterSource(source)));
            }
        }
        // `hint::black_box(x)` is `x`: it only hides `x` from an optimizer.
        // libtest's `test::black_box` is the same.
        let krate = tcx.crate_name(def_id.krate);
        if tcx.item_name(def_id).as_str() == "black_box"
            && ((krate == sym::core && std_path(tcx, def_id).contains("hint")) || krate.as_str() == "test")
        {
            return Some(Some(Std::Same));
        }
        // `convert::identity(x)` is `x`, as its name says.
        if diagnostic("box_new") || diagnostic("convert_identity") {
            return Some(Some(Std::Same));
        }
        if diagnostic("box_assume_init_into_vec_unsafe") {
            return Some(Some(Std::VecMacro));
        }
        // `char::from_digit`, `std::char::from_digit` and `from_u32`.
        let char_fn = |name: &str| {
            tcx.crate_name(def_id.krate) == sym::core
                && tcx.item_name(def_id).as_str() == name
                && std_path(tcx, def_id).contains("char")
        };
        if char_fn("from_digit") {
            return Some(Some(Std::FromDigit));
        }
        if char_fn("from_u32") {
            return Some(Some(Std::FromU32));
        }
        // Of a code point its caller checked.
        if char_fn("from_u32_unchecked") {
            return Some(Some(Std::Text(TextOp::CharFromCode)));
        }
        if tcx.crate_name(def_id.krate).as_str() == "serde_json" {
            match tcx.item_name(def_id).as_str() {
                "to_string" => return Some(Some(Std::ToJson(false))),
                "to_string_pretty" => return Some(Some(Std::ToJson(true))),
                "from_str" => return Some(Some(Std::FromJson)),
                _ => {}
            }
        }
        if diagnostic("vec_from_elem") {
            return Some(Some(Std::FromElem));
        }
        if diagnostic("to_string_method") {
            return Some(Some(Std::ToString));
        }
        if diagnostic("option_unwrap") || diagnostic("option_expect") {
            return Some(Some(Std::Unwrap));
        }
        // `format!(..)` is `must_use(format(format_args!(..)))`, and the
        // arguments are a string already (ADR 0034).
        let krate = tcx.crate_name(def_id.krate);
        let name = tcx.item_name(def_id);
        if krate == sym::core
            && let Some(imp) = tcx.inherent_impl_of_assoc(def_id)
            && Num::of(tcx.type_of(imp).instantiate_identity().skip_normalization()).is_some_and(Num::float)
        {
            match name.as_str() {
                "max" => return Some(Some(Std::MaxOf(true))),
                "min" => return Some(Some(Std::MaxOf(false))),
                _ => {}
            }
        }
        if krate == sym::alloc && name.as_str() == "format" && std_path(tcx, def_id).ends_with("fmt::format") {
            return Some(Some(Std::Format));
        }
        if krate == sym::core && name.as_str() == "must_use" {
            return Some(Some(Std::Same));
        }
        if tcx.is_lang_item(def_id, LangItem::Panic) {
            return Some(Some(Std::Panic));
        }
        // `panic!("{}", x)`, which std writes as `panic_display(&x)` (ADR 0189).
        if tcx.is_lang_item(def_id, LangItem::PanicDisplay) {
            return Some(Some(Std::PanicDisplay { unreachable: false }));
        }
        if krate == sym::core && name.as_str() == "unreachable_display" && std_path(tcx, def_id).contains("panicking") {
            return Some(Some(Std::PanicDisplay { unreachable: true }));
        }
        if krate == sym::std {
            match std_path(tcx, def_id).as_str() {
                "std::io::_print" => return Some(Some(Std::Print { error: false })),
                "std::io::_eprint" => return Some(Some(Std::Print { error: true })),
                "std::rt::begin_panic" => return Some(Some(Std::BeginPanic)),
                _ => {}
            }
        }
        if tcx.is_lang_item(def_id, LangItem::PanicFmt) {
            return Some(Some(Std::PanicFmt));
        }
        // `hint::unreachable_unchecked()`, which a program that's right never
        // reaches: what MIR's unreachable code is.
        if krate == sym::core && std_path(tcx, def_id) == "std::hint::unreachable_unchecked" {
            return Some(Some(Std::Unreachable));
        }
        if tcx.crate_name(def_id.krate) == sym::core && tcx.item_name(def_id).as_str() == "assert_failed" {
            return Some(Some(Std::AssertFailed));
        }
        if diagnostic("deref_method") || diagnostic("deref_mut_method") {
            // A reference to what's inside is the same JS value (ADR 0024 for JS objects).
            let Some(ty) = self_ty else { return Some(None) };
            // What a `Cow` borrows or owns (ADRs 0172, 0319).
            if diagnostic("deref_method") && self.is_cow(ty) {
                return Some(Some(Std::Cow(CowOp::Deref)));
            }
            // A `LazyCell`'s value, made the first time (ADR 0318).
            if self.is_std_type(ty, StdItem::LazyCell) {
                let op = if diagnostic("deref_method") {
                    LazyOp::Force
                } else {
                    LazyOp::ForceMut
                };
                return Some(Some(Std::Lazy(op)));
            }
            // A `Box`'s, and a shared reference's, the value it is (ADR
            // 0023), which generic code's `deref` calls (ADR 0394).
            let boxed =
                diagnostic("deref_method") && (ty.is_box() || matches!(ty.kind(), ty::Ref(_, _, Mutability::Not)));
            let same = self.is_string_like(ty)
                || self.is_path_like(ty)
                // A `ManuallyDrop`'s, the value it is (ADR 0406).
                || self.is_lang_adt(ty, LangItem::ManuallyDrop)
                || self.is_js_object(ty)
                || self.is_rc(ty)
                || self.is_vec_like(ty)
                || boxed;
            // A `PeekMut`'s heap's top (ADR 0333).
            if self.peek_mut_of(ty).is_some() {
                return Some(Some(Std::Heap(HeapOp::PeekTop {
                    mutable: diagnostic("deref_mut_method"),
                })));
            }
            // What a `Pin`'s pointer points at (ADR 0329).
            if self.pinned(ty).is_some() {
                return Some(Some(Std::Pin(match diagnostic("deref_mut_method") {
                    true => PinOp::Mut,
                    false => PinOp::Ref,
                })));
            }
            // What a guard guards, its cell's `value` (ADR 0328).
            if self.is_guard(ty) {
                return Some(Some(Std::GuardValue {
                    mutable: diagnostic("deref_mut_method"),
                }));
            }
            return Some(same.then_some(Std::Same));
        }
        None
    }

    /// `v[i]`, `&v[a..b]` or `&mut v[a..b]`, of a map, a slice, an array, a
    /// `Vec` or text: `Index::index` or `IndexMut::index_mut` of `ty`.
    fn classify_index(&self, trait_: DefId, ty: Ty<'tcx>, args: ty::GenericArgsRef<'tcx>) -> Option<Std> {
        let tcx = self.tcx;
        let items =
            |ty: Ty<'tcx>| ty.peel_refs().is_slice() || ty.peel_refs().is_array() || self.is_vec_like(ty.peel_refs());
        // `m[k]` of a map: its value, or a panic, as `get(k).expect(..)`.
        if tcx.is_lang_item(trait_, LangItem::Index) && self.is_map(ty) && !self.is_set(ty) {
            return Some(Std::Map(MapOp::Index));
        }
        // `&v[a..b]` of a slice, an array or a `Vec` (ADR 0063).
        if tcx.is_lang_item(trait_, LangItem::Index)
            && let Some(range) = args.types().nth(1)
            && self.range_kind(range).is_some()
            && items(ty)
        {
            return Some(Std::Text(TextOp::Slice));
        }
        // `&mut v[a..b]` of a slice, an array or a `Vec`: a view of those
        // items (ADR 0335).
        if tcx.is_lang_item(trait_, LangItem::IndexMut)
            && let Some(range) = args.types().nth(1)
            && self.range_kind(range).is_some()
            && items(ty)
        {
            return Some(Std::Slice(SliceOp::View { checked: false }));
        }
        // `&s[a..b]` of a string: by its UTF-8 bytes (ADR 0138).
        if tcx.is_lang_item(trait_, LangItem::Index)
            && let Some(range) = args.types().nth(1)
            && self.range_kind(range).is_some()
            && self.is_string_like(ty)
        {
            return Some(Std::Text(TextOp::StrSlice));
        }
        // `&mut s[a..b]` of a string: that part of it (ADR 0334).
        if tcx.is_lang_item(trait_, LangItem::IndexMut)
            && let Some(range) = args.types().nth(1)
            && self.range_kind(range).is_some()
            && self.is_string_like(ty)
        {
            return Some(Std::Text(TextOp::StrPart { bounds: false }));
        }
        // `v[i]` of a `Vec` is a slice's, checked the same way.
        if (tcx.is_lang_item(trait_, LangItem::Index) || tcx.is_lang_item(trait_, LangItem::IndexMut))
            && self.is_vec_like(ty.peel_refs())
            && args.types().nth(1).is_some_and(|i| i.is_usize())
        {
            return Some(Std::Index);
        }
        // Of a type parameter's index, `I: SliceIndex<[T]>`, smallvec's
        // `Index<I>`: its dictionary's (ADR 0418).
        let mutable = tcx.is_lang_item(trait_, LangItem::IndexMut);
        if (tcx.is_lang_item(trait_, LangItem::Index) || mutable)
            && args.types().nth(1).is_some_and(|index| self.is_unknown(index))
            && items(ty)
        {
            return Some(Std::Slice(SliceOp::GenericIndex { mutable }));
        }
        None
    }

    /// A trait's method: an operator's, a comparison's, an iterator's, a
    /// conversion's.
    fn classify_trait_method(&self, def_id: DefId, trait_: DefId, args: ty::GenericArgsRef<'tcx>) -> Option<Std> {
        let tcx = self.tcx;
        let self_ty = args.types().next();
        let ty = self_ty?;
        // `x.type_id()`, its type's key (ADR 0331).
        if tcx.is_diagnostic_item(Symbol::intern("Any"), trait_) && tcx.item_name(def_id).as_str() == "type_id" {
            return Some(Std::Any(AnyOp::TypeIdOfValue));
        }
        // `x.borrow()` of std's `Borrow`: the value itself, as its dictionary's
        // is (ADR 0167). A type parameter's is its dictionary's.
        if is_std_def(tcx, trait_, StdItem::Borrow)
            && tcx.item_name(def_id).as_str() == "borrow"
            && self.borrows_as_itself(ty, args.type_at(1))
        {
            return Some(Std::Same);
        }
        // `b.as_ref()` of std's `AsRef` where it's the same JS value, a `Box`'s
        // of what it holds (ADR 0331), or text's as a `Path` (ADR 0173).
        if is_std_def(tcx, trait_, StdItem::AsRef)
            && tcx.item_name(def_id).as_str() == "as_ref"
            && (ty.is_box() || self.is_path_like(args.type_at(1)))
            && self.borrows_as_itself(ty, args.type_at(1))
        {
            return Some(Std::Same);
        }
        // `std::num::Wrapping`'s: its number's, wrapped as release Rust wraps
        // them (ADR 0175).
        if self.wrapping_of(ty.peel_refs()).is_some() {
            if let Some(op) = value_operator(tcx, trait_) {
                return Some(Std::WrappingOp(op, false));
            }
            if let Some(op) = assign_operator(tcx, trait_) {
                return Some(Std::WrappingOp(Ok(op), true));
            }
        }
        if Num::of(ty.peel_refs()).is_some() || ty.peel_refs().is_bool() {
            let operators = [(LangItem::Neg, UnOp::Neg), (LangItem::Not, UnOp::Not)];
            if let Some(&(_, op)) = operators.iter().find(|(item, _)| tcx.is_lang_item(trait_, *item)) {
                return Some(Std::UnaryOperator(op));
            }
        }
        // A clock's `+` and `-`, its only operators (ADR 0447).
        if let Some(clock) = clock_type_of(ty.peel_refs()) {
            return self.clock_operator(trait_, args, clock);
        }
        // A `Duration`'s `+` and `-`, which panic past it as std's do; its
        // other operators aren't its number's (ADR 0188).
        if is_duration_ty(ty.peel_refs()) {
            if tcx.is_lang_item(trait_, LangItem::Add) {
                return Some(Std::Duration(DurationOp::Add));
            }
            if tcx.is_lang_item(trait_, LangItem::Sub) {
                return Some(Std::Duration(DurationOp::Sub));
            }
            if value_operator(tcx, trait_).is_some() || assign_operator(tcx, trait_).is_some() {
                return None;
            }
        }
        if Num::of(ty.peel_refs()).is_some() {
            let operators = [
                (LangItem::Add, BinOp::Add),
                (LangItem::Sub, BinOp::Sub),
                (LangItem::Mul, BinOp::Mul),
                (LangItem::Div, BinOp::Div),
                (LangItem::Rem, BinOp::Rem),
                (LangItem::BitAnd, BinOp::BitAnd),
                (LangItem::BitOr, BinOp::BitOr),
                (LangItem::BitXor, BinOp::BitXor),
                (LangItem::Shl, BinOp::Shl),
                (LangItem::Shr, BinOp::Shr),
            ];
            if let Some(&(_, op)) = operators.iter().find(|(item, _)| tcx.is_lang_item(trait_, *item)) {
                return Some(Std::Operator(op));
            }
            // `total += x` with a `&u32` `x`: the same assignment as with a `u32`.
            let assigning = [
                (LangItem::AddAssign, BinOp::Add),
                (LangItem::SubAssign, BinOp::Sub),
                (LangItem::MulAssign, BinOp::Mul),
                (LangItem::DivAssign, BinOp::Div),
                (LangItem::RemAssign, BinOp::Rem),
                (LangItem::BitAndAssign, BinOp::BitAnd),
                (LangItem::BitOrAssign, BinOp::BitOr),
                (LangItem::BitXorAssign, BinOp::BitXor),
                (LangItem::ShlAssign, BinOp::Shl),
                (LangItem::ShrAssign, BinOp::Shr),
            ];
            if let Some(&(_, op)) = assigning.iter().find(|(item, _)| tcx.is_lang_item(trait_, *item)) {
                return Some(Std::AssignOperator(op));
            }
            if tcx.is_lang_item(trait_, LangItem::PartialOrd) {
                return Some(Std::Operator(match tcx.item_name(def_id).as_str() {
                    "lt" => BinOp::Lt,
                    "le" => BinOp::Le,
                    "gt" => BinOp::Gt,
                    "ge" => BinOp::Ge,
                    _ => return None,
                }));
            }
        }
        if let Some(index) = self.classify_index(trait_, ty, args) {
            return Some(index);
        }
        if tcx.is_lang_item(trait_, LangItem::Add) {
            return self.is_lang_adt(ty, LangItem::String).then_some(Std::Concat);
        }
        // `s += t` is `s.push_str(t)`.
        if tcx.is_lang_item(trait_, LangItem::AddAssign) && self.is_lang_adt(ty.peel_refs(), LangItem::String) {
            return Some(Std::PushStr);
        }
        // A map's or a set's `into_iter()`: its entries, as an array (ADR 0059).
        // A `for` over one takes the `Map` itself.
        if tcx.is_diagnostic_item(sym::IntoIterator, trait_) && self.is_map(ty) {
            return Some(Std::Map(MapOp::Iter(Part::Entries)));
        }
        // An iterator is a JS array (ADR 0036), and a `split` one of strings
        // (ADR 0034). Its adapters are the array's methods.
        // `size_hint()`, where it's known (`classify_size_hint`).
        if tcx.is_diagnostic_item(sym::Iterator, trait_)
            && tcx.item_name(def_id).as_str() == "size_hint"
            && let Some(known) = self.classify_size_hint(ty)
        {
            return Some(known);
        }
        if tcx.is_diagnostic_item(sym::Iterator, trait_) {
            let collects_string = || {
                args.types()
                    .nth(1)
                    .is_some_and(|b| self.is_lang_adt(b, LangItem::String))
            };
            return Some(match tcx.item_name(def_id).as_str() {
                // A `Range`'s moves its `start` (ADR 0129).
                "next" if matches!(self.range_kind(ty.peel_refs()), Some(RangeKind::Exclusive)) => {
                    Std::Range(RangeOp::Next)
                }
                // One of the crate's own is its impl's `next` (ADR 0055).
                "next" if !self.is_user_iterator(ty) => Std::Step(StepOp::Next),
                // The iterator itself, which knows where it is (ADR 0071).
                "by_ref" if !self.is_user_iterator(ty) => Std::IterByRef,
                name if let Some(step) = self.iterator_step(name, ty) => Std::Step(step),
                "map" => Std::ArrayMethod("map"),
                "filter" => Std::ArrayMethod("filter"),
                "any" => Std::ArrayMethod("some"),
                "all" => Std::ArrayMethod("every"),
                "find" => Std::ArrayMethod("find"),
                "for_each" => Std::ArrayMethod("forEach"),
                "enumerate" => Std::Enumerate,
                "rev" => Std::Rev,
                "skip" => Std::Skip,
                "take" => Std::Take,
                "fold" => Std::Fold,
                "sum" => Std::Sum,
                "position" => Std::Position,
                "max" => Std::Extreme(true),
                "min" => Std::Extreme(false),
                "last" => Std::Last,
                "count" => Std::Len,
                name if let Some(comb) = methods::iterator(name) => Std::IterComb(comb),
                "copied" | "cloned" => Std::Cloned,
                "collect" if collects_string() => Std::CollectString,
                "collect" if args.types().nth(1).is_some_and(|b| self.is_map(b)) => {
                    let set = args.types().nth(1).is_some_and(|b| self.is_set(b));
                    Std::Map(MapOp::From { set })
                }
                "collect" => Std::Collect,
                _ => return None,
            });
        }
        // An iterator's `len()` is its `count()`, without taking its items.
        if std_path(tcx, trait_) == "std::iter::ExactSizeIterator"
            && tcx.item_name(def_id).as_str() == "len"
            && self.is_user_iterator(ty)
        {
            return Some(Std::ExactLen);
        }
        // Of what only a caller knows too, `I` or `H::Item`: an array or a JS
        // iterator (ADR 0061), never a collection of the crate's (ADR 0160).
        if tcx.is_diagnostic_item(sym::IntoIterator, trait_)
            && tcx.item_name(def_id).as_str() == "into_iter"
            && (ty.peel_refs().is_array()
                || ty.peel_refs().is_slice()
                || self.is_vec_like(ty.peel_refs())
                || self.is_given_iterable(ty.peel_refs()))
        {
            return Some(Std::Same);
        }
        // 1.99's of a `Box<[T; N]>` or a `Box<[T]>`, which is its array:
        // through a `&mut`, a handle on the box (ADRs 0099, 0388).
        if tcx.is_diagnostic_item(sym::IntoIterator, trait_)
            && tcx.item_name(def_id).as_str() == "into_iter"
            && self.is_boxed_items(ty.peel_refs())
        {
            return Some(Std::Pointee);
        }
        // A range is an iterator already, and its `len` and `next_back`
        // are its bounds' (ADR 0129).
        if let Some(kind) = self.range_kind(ty) {
            let stepped = matches!(kind, RangeKind::Exclusive | RangeKind::Inclusive);
            let name = tcx.item_name(def_id);
            match name.as_str() {
                "into_iter" if tcx.is_diagnostic_item(sym::IntoIterator, trait_) => return Some(Std::Same),
                "len" if stepped && std_path(tcx, trait_) == "std::iter::ExactSizeIterator" => {
                    return Some(Std::Range(RangeOp::Len));
                }
                "next_back"
                    if kind == RangeKind::Exclusive
                        && tcx.get_diagnostic_item(Symbol::intern("DoubleEndedIterator")) == Some(trait_) =>
                {
                    return Some(Std::Range(RangeOp::NextBack));
                }
                _ => {}
            }
        }
        // An `Option`'s, or a `&Option`'s: a `&mut` one's items are places.
        if tcx.is_diagnostic_item(sym::IntoIterator, trait_)
            && tcx.item_name(def_id).as_str() == "into_iter"
            && !matches!(ty.kind(), ty::Ref(_, _, Mutability::Mut))
            && self.is_lang_adt(ty.peel_refs(), LangItem::Option)
        {
            return Some(Std::OptionIter);
        }
        if tcx.is_diagnostic_item(sym::IntoIterator, trait_)
            && tcx.item_name(def_id).as_str() == "into_iter"
            && self.is_lang_adt(ty.peel_refs(), LangItem::Option)
        {
            return Some(Std::OptionIterMut);
        }
        // `cmp`, `max` and `min` of what JS's `<` orders the same way.
        if tcx.is_diagnostic_item(sym::Ord, trait_) {
            let peeled = ty.peel_refs();
            let comparable = Num::of(peeled).is_some() || peeled.is_bool() || self.is_string_like(peeled);
            return match tcx.item_name(def_id).as_str() {
                "cmp" if comparable => Some(Std::Cmp),
                "max" if Num::of(peeled).is_some() => Some(Std::MaxOf(true)),
                "min" if Num::of(peeled).is_some() => Some(Std::MaxOf(false)),
                "clamp" if Num::of(peeled).is_some() => Some(Std::Number(NumOp::Clamp)),
                _ => None,
            };
        }
        // `write!(s, ..)` into a `String`: `s += ..`, as `push_str` is; writing
        // to a string can't fail, so its `fmt::Result` is nothing (ADR 0148).
        if self.is_lang_adt(ty.peel_refs(), LangItem::String)
            && tcx.is_diagnostic_item(Symbol::intern("FmtWrite"), trait_)
        {
            return match tcx.item_name(def_id).as_str() {
                "write_fmt" | "write_str" | "write_char" => Some(Std::PushStr),
                _ => None,
            };
        }
        // A writer of the crate's own: what `write!` and `write_char` give its
        // `write_str` (ADR 0166). One it writes itself is called as it is.
        if tcx.is_diagnostic_item(Symbol::intern("FmtWrite"), trait_)
            && matches!(tcx.item_name(def_id).as_str(), "write_fmt" | "write_char")
            && self.has_user_impl(trait_, ty.peel_refs())
        {
            return Some(Std::UserWrite);
        }
        // Writing to a standard stream or a `Vec<u8>` (ADRs 0132, 0392).
        if let Some(write) = self.classify_io_write(def_id, trait_, ty) {
            return Some(write);
        }
        if is_extend(tcx, trait_)
            && let Some(extend) = self.std_extend(ty.peel_refs())
        {
            return Some(extend);
        }
        // `String::from_iter(items)` is `items.collect()` into a `String`.
        if tcx.is_diagnostic_item(sym::FromIterator, trait_) && self.is_lang_adt(ty, LangItem::String) {
            return Some(Std::CollectString);
        }
        // A `From` or an `Into`: std's own conversions.
        if let Some(conversion) = self.classify_conversion(trait_, args) {
            return Some(conversion);
        }
        // Between integers, which may not fit.
        let into = tcx.is_diagnostic_item(sym::TryInto, trait_);
        let sides =
            conversion_sides(tcx, trait_, args).filter(|_| into || tcx.is_diagnostic_item(sym::TryFrom, trait_));
        let (from_ty, to_ty) = (sides.map(|(from, _)| from), sides.map(|(_, to)| to));
        if let (Some(from), Some(to)) = (from_ty.and_then(|t| Num::of(t.peel_refs())), to_ty.and_then(Num::of))
            && !from.float()
            && !to.float()
        {
            return Some(Std::TryFromInt { into });
        }
        // A slice into an array of its length, shared or copied (ADR 0189).
        if let (Some(from), Some(to)) = (from_ty, to_ty)
            && matches!(from.kind(), ty::Ref(_, slice, Mutability::Not) if slice.is_slice())
            && match to.kind() {
                ty::Ref(_, array, Mutability::Not) => array.is_array(),
                _ => to.is_array(),
            }
        {
            return Some(Std::SliceToArray { into });
        }
        let from_str = tcx.is_diagnostic_item(sym::From, trait_) && self.is_lang_adt(ty, LangItem::String);
        let to_owned = tcx.is_diagnostic_item(Symbol::intern("ToOwned"), trait_);
        // A slice's `to_owned()` is its `to_vec()`.
        (from_str || to_owned && ty.is_str())
            .then_some(Std::Same)
            .or((to_owned && ty.is_slice()).then_some(Std::ToVec))
    }

    /// `size_hint()` of an iterator of the crate's that keeps std's, of std's
    /// that knows its length (ADR 0170), or of a range, its bounds' (ADR
    /// 0129).
    fn classify_size_hint(&self, ty: Ty<'tcx>) -> Option<Std> {
        if self.is_user_iterator(ty) {
            return Some(Std::SizeHint(false));
        }
        match self.range_kind(ty.peel_refs()) {
            Some(RangeKind::Exclusive | RangeKind::Inclusive) => Some(Std::Range(RangeOp::SizeHint)),
            None if self.is_array_iter(ty.peel_refs()) => Some(Std::SizeHint(true)),
            _ => None,
        }
    }

    /// A write, `io::Write`'s, to a standard stream, `print!`'s (ADR 0132),
    /// or to a `Vec<u8>`, its bytes added, as `extend_from_slice` adds
    /// them (ADR 0392), where the writer's type is known.
    fn classify_io_write(&self, def_id: DefId, trait_: DefId, ty: Ty<'tcx>) -> Option<Std> {
        if std_path(self.tcx, trait_) != "std::io::Write" {
            return None;
        }
        // A writer of bytes of the crate's own that keeps std's `write_all`
        // and `write_fmt`: its dictionary's (ADR 0392).
        if io_write_default(self.tcx, def_id).is_some()
            && matches!(ty.peel_refs().kind(), ty::Adt(..))
            && self.has_user_impl(trait_, ty.peel_refs())
        {
            return Some(Std::UserIoWrite);
        }
        let name = self.tcx.item_name(def_id);
        if let Some(error) = self.stream(ty.peel_refs()) {
            return match name.as_str() {
                "write_fmt" => Some(Std::Stream(StreamOp::Write { error })),
                "write_all" => Some(Std::Stream(StreamOp::WriteBytes { error })),
                "write" => Some(Std::Stream(StreamOp::Written { error })),
                "flush" => Some(Std::Stream(StreamOp::Nothing)),
                _ => None,
            };
        }
        if self.io_writer(ty) != Some("$vecWriter") {
            return None;
        }
        match name.as_str() {
            "write_fmt" => Some(Std::WriteText),
            "write_all" => Some(Std::Comb(Comb::ExtendFromSlice)),
            "flush" => Some(Std::Nothing),
            _ => None,
        }
    }

    /// A `From`'s or an `Into`'s conversion, of std's, if rust-js knows it:
    /// one list of what it converts into what, `conversion_sides`', the same
    /// of either.
    fn classify_conversion(&self, trait_: DefId, args: ty::GenericArgsRef<'tcx>) -> Option<Std> {
        let tcx = self.tcx;
        let into = tcx.is_diagnostic_item(sym::Into, trait_);
        if !into && !tcx.is_diagnostic_item(sym::From, trait_) {
            return None;
        }
        let (from_ty, to_ty) = conversion_sides(tcx, trait_, args)?;
        // `From<T> for T`, std's for every type, and the `Into` it gives:
        // the value itself, of an `impl Into<T>` that is a `T` too (ADR 0387).
        if tcx.erase_and_anonymize_regions(self.reveal(from_ty)) == tcx.erase_and_anonymize_regions(self.reveal(to_ty))
        {
            return Some(Std::Same);
        }
        // `VecDeque::from(v)` is a copy of `v`, which may be a clone that
        // was never made (ADR 0052), and so is `Vec::from([..])`;
        // `BinaryHeap::from(v)` puts one in heap order.
        if self.is_std_type(to_ty, StdItem::VecDeque)
            || self.is_std_type(to_ty, StdItem::LinkedList)
            || (self.is_std_type(to_ty, StdItem::Vec) && from_ty.is_array())
        {
            return Some(Std::ToVec);
        }
        if self.is_std_type(to_ty, StdItem::BinaryHeap) {
            return Some(Std::Heap(HeapOp::From));
        }
        // `HashMap::from([(k, v)])`: `new Map([[k, v]])`.
        if self.is_map(to_ty) {
            return Some(Std::Map(MapOp::From {
                set: self.is_set(to_ty),
            }));
        }
        // `Option::from(x)`: `Some(x)`.
        if self.option_of(to_ty) == Some(from_ty) {
            return Some(Std::OptionFrom);
        }
        // A `Cow` of a reference to what it borrows, or to what owns it,
        // `Borrowed`, or of what owns, `Owned`.
        if let Some((borrowed, owned)) = self.cow_parts(to_ty) {
            let erased = |ty: Ty<'tcx>| tcx.erase_and_anonymize_regions(ty);
            let of_owned = erased(from_ty) == erased(owned);
            let lent = matches!(from_ty.kind(), ty::Ref(_, to, Mutability::Not)
                if erased(*to) == erased(borrowed) || erased(*to) == erased(owned));
            if of_owned || lent {
                return Some(Std::Cow(CowOp::From { owned: of_owned }));
            }
        }
        // An `Rc` or an `Arc` of the value, which `rc_same` says it makes, as
        // `Rc::new(x)`: of a `str` its text, of a `[T]` a `Vec<T>`'s items.
        if rc_made_of(tcx, from_ty, to_ty).is_some() {
            return Some(Std::Same);
        }
        let (from_ty, to_ty) = (Some(from_ty), Some(to_ty));
        // std's own conversions that change nothing in JS (ADR 0063): to a
        // `String` or a `Box<str>` (ADR 0191) from a `&str` or a `char`, and
        // between numbers, which only widen.
        if let (Some(from_ty), Some(to_ty)) = (from_ty, to_ty) {
            let boxed_str = to_ty.is_box() && to_ty.boxed_ty().is_some_and(|inner| inner.is_str());
            if (self.is_lang_adt(to_ty, LangItem::String) || self.is_path_like(to_ty) || boxed_str)
                && self.is_string_like(from_ty)
            {
                return Some(Std::Same);
            }
            // A `char` from a `u8`: the code point it is (ADR 0157).
            if to_ty.is_char() && matches!(from_ty.kind(), ty::Uint(ty::UintTy::U8)) {
                return Some(Std::Text(TextOp::CharFromByte));
            }
            // Into a `Box`, which is its value (ADR 0023): `Box::from(x)`, and
            // a `Vec`'s items as a boxed slice.
            if let ty::Adt(_, boxed) = to_ty.kind()
                && to_ty.is_box()
                && (boxed.type_at(0) == from_ty
                    || matches!((boxed.type_at(0).kind(), from_ty.kind()), (ty::Slice(item), ty::Adt(_, vec))
                        if self.is_std_type(from_ty, StdItem::Vec) && vec.type_at(0) == *item))
            {
                return Some(Std::Same);
            }
            // A number from a `bool` or a `char`, as `as` makes it (ADR 0358).
            if (from_ty.is_bool() || from_ty.is_char()) && Num::of(to_ty).is_some() {
                return Some(Std::Cast { into });
            }
            // Into a BigInt from a number (ADR 0086), else the same.
            if let (Some(from), Some(to)) = (Num::of(from_ty.peel_refs()), Num::of(to_ty)) {
                return Some(if to.big() && !from.big() { Std::ToBig } else { Std::Same });
            }
        }
        None
    }

    /// A method of a std type's own, by its type and its name.
    fn classify_inherent(&self, def_id: DefId, args: ty::GenericArgsRef<'tcx>) -> Option<Std> {
        let tcx = self.tcx;
        let self_ty = args.types().next();
        let owner = tcx
            .type_of(tcx.inherent_impl_of_assoc(def_id)?)
            .instantiate_identity()
            .skip_normalization();
        let adt = |item: StdItem| self.is_std_type(owner, item);
        let string = self.is_lang_adt(owner, LangItem::String);
        let option = self.is_lang_adt(owner, LangItem::Option);
        let range = self.range_kind(owner);
        let result = self.is_std_type(owner, StdItem::Result);
        let ordering = self.is_lang_adt(owner, LangItem::OrderingEnum);
        let local_key = adt(StdItem::LocalKey);
        let once = self.is_std_type(owner, StdItem::OnceCell);
        let rc = rc_pointee(tcx, owner).is_some();
        let weak = weak_pointee(tcx, owner).is_some();
        let lazy = self.is_std_type(owner, StdItem::LazyCell);
        let cow = self.is_cow(owner);
        let arguments = self.is_lang_adt(owner, LangItem::FormatArguments);
        let argument = self.is_lang_adt(owner, LangItem::FormatArgument);
        let of = MapOwner {
            map: adt(StdItem::HashMap) || adt(StdItem::BTreeMap) || self.is_json_map(owner),
            set: adt(StdItem::HashSet) || adt(StdItem::BTreeSet),
            std_map: adt(StdItem::HashMap) || adt(StdItem::BTreeMap),
            btree_map: adt(StdItem::BTreeMap),
            placed: matches!(owner.kind(), ty::Adt(adt, _) if is_index_map(tcx, adt.did())),
            entry: adt(StdItem::HashMapEntry) || adt(StdItem::BTreeEntry),
            occupied: self.is_occupied_entry(owner),
            vacant: self.is_vacant_entry(owner),
        };
        let name = tcx.item_name(def_id);
        if (adt(StdItem::Cell) || adt(StdItem::RefCell))
            && let Some(cell) = cell_method(name.as_str(), adt(StdItem::RefCell))
        {
            return Some(cell);
        }
        // A clock's: of its nanoseconds (ADR 0447).
        if let Some(clock) = clock_type_of(owner) {
            return clock_method(clock, name.as_str());
        }
        // A `Duration`'s: of its nanoseconds (ADR 0188).
        if is_duration_ty(owner) {
            const SECOND: i64 = 1_000_000_000;
            return Some(Std::Duration(match name.as_str() {
                "new" => DurationOp::New,
                "from_secs" => DurationOp::From(SECOND),
                "from_millis" => DurationOp::From(1_000_000),
                "from_micros" => DurationOp::From(1_000),
                "from_nanos" => DurationOp::From(1),
                "as_secs" => DurationOp::As(SECOND),
                "as_millis" => DurationOp::As(1_000_000),
                "as_micros" => DurationOp::As(1_000),
                "as_nanos" => DurationOp::As(1),
                "subsec_nanos" => DurationOp::Subsec(1),
                "subsec_micros" => DurationOp::Subsec(1_000),
                "subsec_millis" => DurationOp::Subsec(1_000_000),
                "is_zero" => DurationOp::IsZero,
                "checked_add" => DurationOp::Checked { add: true },
                "checked_sub" => DurationOp::Checked { add: false },
                _ => return None,
            }));
        }
        // A `LinkedList` is an array as a deque is (ADR 0316).
        let (deque, heap) = (
            adt(StdItem::VecDeque) || adt(StdItem::LinkedList),
            adt(StdItem::BinaryHeap),
        );
        // Theirs first: `push` and `pop` keep a heap's order, and a deque's
        // `remove` is an `Option` (ADR 0068).
        let peekable = self.is_peekable(owner);
        let chars = self.is_str_chars(owner);
        let slice_iter = adt(StdItem::SliceIter)
            || matches!(owner.kind(), ty::Adt(it, _) if std_path(tcx, it.did()) == "std::vec::IntoIter");
        let own = match name.as_str() {
            "send" if self.channel_end(owner) == Some(ChannelEnd::Sender) => Some(Std::Channel(ChannelOp::Send)),
            "recv" if self.channel_end(owner) == Some(ChannelEnd::Receiver) => Some(Std::Channel(ChannelOp::Recv)),
            "try_recv" if self.channel_end(owner) == Some(ChannelEnd::Receiver) => {
                Some(Std::Channel(ChannelOp::TryRecv))
            }
            "peek" if peekable => Some(Std::Step(StepOp::Peek)),
            "next_if" if peekable => Some(Std::Step(StepOp::NextIf)),
            "next_if_eq" if peekable => Some(Std::Step(StepOp::NextIfEq)),
            "as_str" if chars => Some(Std::Step(StepOp::AsStr)),
            "as_slice" if slice_iter => Some(Std::Step(StepOp::AsSlice)),
            "push" if heap => Some(Std::Heap(HeapOp::Push)),
            "pop" if heap => Some(Std::Heap(HeapOp::Pop)),
            "peek" if heap => Some(Std::First),
            "into_sorted_vec" if heap => Some(Std::Heap(HeapOp::IntoSorted)),
            "into_vec" if heap => Some(Std::Same),
            // In the order std's heap keeps them (ADR 0333).
            "append" if heap => Some(Std::Heap(HeapOp::Append)),
            "retain" if heap => Some(Std::Heap(HeapOp::Retain)),
            "drain" if heap => Some(Std::Heap(HeapOp::Drain)),
            "peek_mut" if heap => Some(Std::Heap(HeapOp::PeekMut)),
            "as_slice" if heap => Some(Std::Same),
            "remove" if deque => Some(Std::DequeRemove),
            "extract_if" if adt(StdItem::Vec) || adt(StdItem::LinkedList) => Some(Std::Slice(SliceOp::ExtractIf {
                range: adt(StdItem::Vec),
            })),
            // A `VecDeque`'s `insert`, checked with its own message, and the
            // `_mut` ones' `&mut` to what they put in (ADR 0333).
            "insert" | "insert_mut" if adt(StdItem::VecDeque) => Some(Std::Slice(SliceOp::DequeInsert {
                mutable: name.as_str() == "insert_mut",
            })),
            "push_back_mut" if deque => Some(Std::Slice(SliceOp::PushMut { at: false })),
            "push_front_mut" if deque => Some(Std::Slice(SliceOp::PushFrontMut)),
            "push_back" if deque => Some(Std::Push),
            "pop_back" if deque => Some(Std::Method("pop")),
            "push_front" if deque => Some(Std::Method("unshift")),
            "pop_front" if deque => Some(Std::Method("shift")),
            "front" | "front_mut" if deque => Some(Std::First),
            "back" | "back_mut" if deque => Some(Std::SliceLast),
            "make_contiguous" if deque => Some(Std::Same),
            // A JS array has no capacity: asking for some does nothing (ADR
            // 0315). A `Vec`'s is `capacity_op`'s (ADR 0419).
            "reserve" | "reserve_exact" | "shrink_to" | "shrink_to_fit" if deque || heap || of.map || of.set => {
                Some(Std::Nothing)
            }
            "into_boxed_slice" if adt(StdItem::Vec) => Some(Std::Same),
            // A JS string has no capacity either (ADR 0322).
            "reserve" | "reserve_exact" | "shrink_to" | "shrink_to_fit" if string => Some(Std::Nothing),
            // A `Box<str>` is its string, and a `Box<[T]>` its array.
            "into_boxed_str" if string => Some(Std::Same),
            // A `String`'s bytes, a new array, as a `str`'s are.
            "as_bytes" | "into_bytes" if string => Some(Std::Text(TextOp::Bytes)),
            "into_string" if owner.is_str() => Some(Std::Same),
            "into_vec" if owner.is_slice() => Some(Std::Same),
            "into_flattened" if adt(StdItem::Vec) => Some(Std::Comb(Comb::Concat)),
            "splice" if adt(StdItem::Vec) => Some(Std::Text(TextOp::Splice)),
            "extend_from_within" if adt(StdItem::Vec) => Some(Std::Text(TextOp::ExtendFromWithin)),
            // A `Vec` is its array, and the whole of it the same array: what's
            // written through `as_mut_slice()` is the `Vec`'s. An array's is it too.
            "as_slice" | "as_mut_slice" if adt(StdItem::Vec) || owner.is_array() => Some(Std::Same),
            "drain" if adt(StdItem::Vec) || deque => Some(Std::Text(TextOp::Drain)),
            "iter" | "iter_mut" if deque || heap => Some(Std::Same),
            "new" if deque || heap => Some(Std::VecNew),
            "len" if deque || heap => Some(Std::Len),
            "is_empty" if deque || heap => Some(Std::IsEmpty),
            "clear" if deque || heap => Some(Std::Clear),
            "retain" if deque => Some(Std::Retain),
            _ => None,
        };
        if own.is_some() {
            return own;
        }
        if let Some(op) = methods::text(name.as_str(), owner.is_char(), owner.is_str()) {
            return Some(Std::Text(op));
        }
        if let Some(num) = Num::of(owner)
            && let Some(op) = methods::number(name.as_str(), num)
        {
            return Some(Std::Number(op));
        }
        if let Some(comb) = methods::combinator(
            name.as_str(),
            option,
            result,
            adt(StdItem::Vec) || deque,
            adt(StdItem::Vec) || deque || owner.is_slice(),
        )
        .or_else(|| methods::boolean(name.as_str(), owner.is_bool()))
        {
            // A `VecDeque`'s assertion calls the count `n`.
            let comb = match comb {
                Comb::Rotate { left, .. } if deque => Comb::Rotate { left, count: "n" },
                comb => comb,
            };
            return Some(Std::Comb(comb));
        }
        Some(match tcx.item_name(def_id).as_str() {
            // A map's, a set's or an entry's (ADRs 0059, 0325, 0345).
            name if let Some(op) = map_method(name, of) => Std::Map(op),
            "from_str" | "from_str_nonconst" if arguments => Std::FmtStr,
            "new" if arguments => Std::FmtNew,
            "new_display" if argument => Std::FmtDisplay,
            "new_debug" if argument => Std::FmtDebug,
            "new_lower_hex" if argument => Std::FmtRadix(Radix::LowerHex),
            "new_upper_hex" if argument => Std::FmtRadix(Radix::UpperHex),
            "new_binary" if argument => Std::FmtRadix(Radix::Binary),
            "new_octal" if argument => Std::FmtRadix(Radix::Octal),
            "new_lower_exp" if argument => Std::FmtExp(false),
            "new_upper_exp" if argument => Std::FmtExp(true),
            "new_pointer" if argument => Std::FmtPointer,
            "from_usize" if argument => Std::FmtUsize,
            "new" if adt(StdItem::Rc) || adt(StdItem::Arc) => Std::Same,
            // What only a counted one answers (ADR 0320).
            "strong_count" if rc => Std::Rc(RcOp::StrongCount),
            "weak_count" if rc => Std::Rc(RcOp::WeakCount),
            "ptr_eq" if rc || weak => Std::Rc(RcOp::PtrEq),
            "downgrade" if rc => Std::Rc(RcOp::Downgrade),
            "get_mut" if rc => Std::Rc(RcOp::GetMut),
            "make_mut" if rc => Std::Rc(RcOp::MakeMut),
            "try_unwrap" if rc => Std::Rc(RcOp::TryUnwrap),
            "into_inner" if rc => Std::Rc(RcOp::IntoInner),
            "unwrap_or_clone" if rc => Std::Rc(RcOp::UnwrapOrClone),
            "new_cyclic" if rc => Std::Rc(RcOp::NewCyclic),
            "new" if weak => Std::Rc(RcOp::WeakNew),
            "upgrade" if weak => Std::Rc(RcOp::Upgrade),
            "strong_count" if weak => Std::Rc(RcOp::WeakStrongCount),
            "weak_count" if weak => Std::Rc(RcOp::WeakWeakCount),
            // Kept for good, a `&'static` of it: in JS, which frees nothing
            // itself, the value itself.
            "leak" if adt(StdItem::Vec) => Std::Same,
            "leak" if string || owner.is_box() => Std::Leak,
            // A `dyn Any`'s downcasts, by its type's key (ADR 0331).
            "is" if self.is_dyn_any(owner) => Std::Any(AnyOp::Is),
            "downcast_ref" if self.is_dyn_any(owner) => Std::Any(AnyOp::DowncastRef),
            "downcast_mut" if self.is_dyn_any(owner) => Std::Any(AnyOp::DowncastMut),
            "downcast"
                if (owner.is_box() || rc)
                    && owner
                        .walk()
                        .any(|part| part.as_type().is_some_and(|t| self.is_dyn_any(t))) =>
            {
                Std::Any(AnyOp::Downcast)
            }
            "of" if self.is_type_id(owner) => Std::Any(AnyOp::TypeIdOf),
            // A `Pin` is its pointer (ADR 0329).
            "pin" | "into_pin" if owner.is_box() => Std::Same,
            "pop" if self.peek_mut_of(owner).is_some() => Std::Heap(HeapOp::PeekPop),
            // What a `Box` or a `MaybeUninit` holds, or `undefined` before it's
            // written (ADR 0332).
            "new_uninit" if owner.is_box() => Std::Uninit(UninitOp::Uninit),
            "new_zeroed" if owner.is_box() => Std::Uninit(UninitOp::Zeroed),
            "new_uninit_slice" if owner.is_box() => Std::Uninit(UninitOp::Slice { zeroed: false }),
            "new_zeroed_slice" if owner.is_box() => Std::Uninit(UninitOp::Slice { zeroed: true }),
            "write" if owner.is_box() => Std::Uninit(UninitOp::BoxWrite),
            "assume_init" if owner.is_box() => Std::Same,
            "uninit" if self.uninit_of(owner).is_some() => Std::Uninit(UninitOp::Uninit),
            "zeroed" if self.uninit_of(owner).is_some() => Std::Uninit(UninitOp::Zeroed),
            "new" | "assume_init" | "assume_init_ref" | "assume_init_read" if self.uninit_of(owner).is_some() => {
                Std::Same
            }
            "write" if self.uninit_of(owner).is_some() => Std::Uninit(UninitOp::Write),
            "assume_init_mut" if self.uninit_of(owner).is_some() => Std::Uninit(UninitOp::InitMut),
            "new" | "new_unchecked" | "into_inner" | "into_inner_unchecked" | "get_ref" | "static_ref"
                if self.pinned(owner).is_some() =>
            {
                Std::Same
            }
            "as_mut" | "get_mut" | "get_unchecked_mut" | "static_mut" if self.pinned(owner).is_some() => {
                Std::Pin(PinOp::Mut)
            }
            "as_ref" | "into_ref" if self.pinned(owner).is_some() => Std::Pin(PinOp::Ref),
            "set" if self.pinned(owner).is_some() => Std::Pin(PinOp::Set),
            "map_unchecked" | "map_unchecked_mut" if self.pinned(owner).is_some() => Std::Pin(PinOp::Map),
            "new"
                if adt(StdItem::Cell)
                    || adt(StdItem::RefCell)
                    || adt(StdItem::Atomic)
                    || adt(StdItem::Mutex)
                    || adt(StdItem::RwLock) =>
            {
                Std::CellNew
            }
            // On one thread a lock is never contested by another: always `Ok`
            // (ADR 0025), but locked again while held, a deadlock (ADR 0328).
            "lock" if adt(StdItem::Mutex) => Std::Borrow {
                mutable: true,
                lock: true,
            },
            "read" | "write" if adt(StdItem::RwLock) => Std::Borrow {
                mutable: name.as_str() == "write",
                lock: true,
            },
            "into_inner" | "get_mut" if adt(StdItem::Mutex) || adt(StdItem::RwLock) => Std::Lock,
            "is_poisoned" if adt(StdItem::Mutex) || adt(StdItem::RwLock) => Std::NotPoisoned,
            "clear_poison" if adt(StdItem::Mutex) || adt(StdItem::RwLock) => Std::Nothing,
            "into_inner" | "get_ref" if is_poison_error(owner) => Std::PoisonData,
            "each_ref" | "each_mut" if owner.is_array() => Std::Same,
            "swap_remove_back" if deque => Std::Slice(SliceOp::DequeSwapRemove { front: false }),
            "swap_remove_front" if deque => Std::Slice(SliceOp::DequeSwapRemove { front: true }),
            "retain_mut" if adt(StdItem::Vec) || deque => Std::Slice(SliceOp::RetainMut),
            "pop_front_if" if deque => Std::Slice(SliceOp::PopFrontIf),
            "partition_point" if deque => Std::Slice(SliceOp::PartitionPoint),
            "range" if deque => Std::Text(TextOp::Slice),
            "range_mut" if deque => Std::Slice(SliceOp::DequeRangeMut),
            "escape_default" | "escape_debug" | "escape_unicode" if owner.is_char() || owner.is_str() => {
                Std::Text(TextOp::Escape {
                    kind: match name.as_str() {
                        "escape_default" => "default",
                        "escape_debug" => "debug",
                        _ => "unicode",
                    },
                    str: owner.is_str(),
                })
            }
            "encode_utf8" if owner.is_char() => Std::Text(TextOp::EncodeUtf8),
            "decode_utf16" if owner.is_char() => Std::Text(TextOp::DecodeUtf16),
            "unpaired_surrogate" if is_decode_utf16_error(tcx, owner) => Std::Same,
            "make_ascii_uppercase" | "make_ascii_lowercase" if owner.is_char() || owner.is_str() => {
                Std::StringEdit(StringEdit::AsciiCase {
                    upper: name.as_str() == "make_ascii_uppercase",
                })
            }
            "borrow" | "borrow_mut" if adt(StdItem::RefCell) => Std::Borrow {
                mutable: name.as_str() == "borrow_mut",
                lock: false,
            },
            "try_lock" if adt(StdItem::Mutex) => Std::TryBorrow {
                mutable: true,
                lock: true,
            },
            "try_read" | "try_write" if adt(StdItem::RwLock) => Std::TryBorrow {
                mutable: name.as_str() == "try_write",
                lock: true,
            },
            "try_borrow" | "try_borrow_mut" if adt(StdItem::RefCell) => Std::TryBorrow {
                lock: false,
                mutable: name.as_str() == "try_borrow_mut",
            },
            "load" | "into_inner" if adt(StdItem::Atomic) => Std::AtomicLoad,
            "store" if adt(StdItem::Atomic) => Std::AtomicStore,
            "swap" if adt(StdItem::Atomic) => Std::AtomicSwap,
            "fetch_add" if adt(StdItem::Atomic) => Std::AtomicFetch(BinOp::Add),
            "fetch_sub" if adt(StdItem::Atomic) => Std::AtomicFetch(BinOp::Sub),
            "fetch_and" if adt(StdItem::Atomic) => Std::AtomicFetch(BinOp::BitAnd),
            "fetch_or" if adt(StdItem::Atomic) => Std::AtomicFetch(BinOp::BitOr),
            "fetch_xor" if adt(StdItem::Atomic) => Std::AtomicFetch(BinOp::BitXor),
            "fetch_max" if adt(StdItem::Atomic) => Std::AtomicFetchMax(true),
            "fetch_min" if adt(StdItem::Atomic) => Std::AtomicFetchMax(false),
            "compare_exchange" | "compare_exchange_weak" if adt(StdItem::Atomic) => Std::AtomicCompareExchange,
            "new" if once => Std::Once(OnceOp::New),
            "get" | "into_inner" if once => Std::Once(OnceOp::Get),
            "get_mut" if once => Std::Once(OnceOp::GetMut),
            "set" if once => Std::Once(OnceOp::Set),
            "get_or_init" if once => Std::Once(OnceOp::GetOrInit),
            "take" if once => Std::Once(OnceOp::Take),
            "into_owned" if cow => Std::Cow(CowOp::IntoOwned),
            "to_mut" if cow => Std::Cow(CowOp::ToMut),
            "new" if lazy => Std::Lazy(LazyOp::New),
            "force" if lazy => Std::Lazy(LazyOp::Force),
            "force_mut" if lazy => Std::Lazy(LazyOp::ForceMut),
            "get" if lazy => Std::Lazy(LazyOp::Get),
            "get_mut" if lazy => Std::Lazy(LazyOp::GetMut),
            "new" if adt(StdItem::Vec) => Std::VecNew,
            "push" if adt(StdItem::Vec) => Std::Push,
            "push_mut" if adt(StdItem::Vec) => Std::Slice(SliceOp::PushMut { at: false }),
            "insert_mut" if adt(StdItem::Vec) => Std::Slice(SliceOp::PushMut { at: true }),
            // JS's `pop()` gives `undefined` when empty: `None` (ADR 0030).
            "pop" if adt(StdItem::Vec) => Std::Method("pop"),
            "len" if adt(StdItem::Vec) || owner.is_slice() => Std::Len,
            // A string counts its UTF-8 bytes, as Rust's does (ADR 0138).
            "len" if owner.is_str() || string => Std::Text(TextOp::ByteLen),
            "char_indices" if owner.is_str() => Std::Text(TextOp::CharIndices),
            "clear" if adt(StdItem::Vec) => Std::Clear,
            "retain" if adt(StdItem::Vec) => Std::Retain,
            "iter" | "iter_mut" if owner.is_slice() => Std::Same,
            "new" if string => Std::StringNew,
            "with_capacity" if string => Std::StringWithCapacity,
            "pop" if string => Std::StringEdit(StringEdit::Pop),
            "remove" if string => Std::StringEdit(StringEdit::Remove),
            "truncate" if string => Std::StringEdit(StringEdit::Truncate),
            "insert" | "insert_str" if string => Std::StringEdit(StringEdit::Insert),
            "retain" if string => Std::StringEdit(StringEdit::Retain),
            "clear" if string => Std::StringEdit(StringEdit::Clear),
            "split_off" if string => Std::StringEdit(StringEdit::SplitOff),
            "drain" if string => Std::StringEdit(StringEdit::Drain),
            "replace_range" if string => Std::StringEdit(StringEdit::ReplaceRange),
            "extend_from_within" if string => Std::StringEdit(StringEdit::ExtendFromWithin),
            "trim_ascii" if owner.is_str() => Std::Text(TextOp::TrimAscii { start: true, end: true }),
            "trim_ascii_start" if owner.is_str() => Std::Text(TextOp::TrimAscii {
                start: true,
                end: false,
            }),
            "trim_ascii_end" if owner.is_str() => Std::Text(TextOp::TrimAscii {
                start: false,
                end: true,
            }),
            "split_at_checked" if owner.is_str() => Std::Text(TextOp::StrSplitAtChecked),
            "encode_utf16" if owner.is_str() => Std::Text(TextOp::EncodeUtf16),
            "floor_char_boundary" if owner.is_str() => Std::Text(TextOp::CharBoundaryNear { ceil: false }),
            "ceil_char_boundary" if owner.is_str() => Std::Text(TextOp::CharBoundaryNear { ceil: true }),
            "from_utf16" if string => Std::Text(TextOp::FromUtf16 { lossy: false }),
            "from_utf16_lossy" if string => Std::Text(TextOp::FromUtf16 { lossy: true }),
            "as_str" | "as_mut_str" if string => Std::Same,
            "trim" if owner.is_str() => Std::Trim { start: true, end: true },
            // A closure, a function or a set of `char`s as the pattern (ADRs 0063, 0157).
            "split" | "splitn" | "contains" if owner.is_str() && self_ty.is_some_and(|p| self.is_char_predicate(p)) => {
                Std::Text(match name.as_str() {
                    "split" => TextOp::SplitBy,
                    "splitn" => TextOp::SplitNBy,
                    _ => TextOp::ContainsBy,
                })
            }
            "find" | "rfind" if owner.is_str() && self_ty.is_some_and(|p| self.is_char_predicate(p)) => {
                Std::Text(TextOp::FindBy(name.as_str() == "rfind"))
            }
            "starts_with" | "ends_with" if owner.is_str() && self_ty.is_some_and(|p| self.is_char_predicate(p)) => {
                Std::Text(TextOp::StartsBy {
                    end: name.as_str() == "ends_with",
                })
            }
            "split_ascii_whitespace" if owner.is_str() => Std::Text(TextOp::SplitAsciiWhitespace),
            "get" if owner.is_str() => Std::Text(TextOp::StrGet),
            "get_mut" if owner.is_str() => Std::Text(TextOp::StrGetMut),
            "get_unchecked" if owner.is_str() => Std::Text(TextOp::StrSlice),
            "get_unchecked_mut" if owner.is_str() => Std::Text(TextOp::StrPart { bounds: false }),
            "slice_unchecked" if owner.is_str() => Std::Text(TextOp::StrSliceBounds),
            "slice_mut_unchecked" if owner.is_str() => Std::Text(TextOp::StrPart { bounds: true }),
            "split_at_mut" | "split_at_mut_checked" if owner.is_str() => Std::Text(TextOp::StrSplitAtMut {
                checked: name.as_str() == "split_at_mut_checked",
            }),
            "get" if owner.is_slice() && args.types().nth(1).is_some_and(|r| self.range_kind(r).is_some()) => {
                Std::Text(TextOp::SliceGet)
            }
            "split_at" | "split_at_checked" if owner.is_slice() => Std::Text(TextOp::SliceSplitAt {
                checked: name.as_str() == "split_at_checked",
            }),
            "split_first" | "split_last" if owner.is_slice() => Std::Text(TextOp::SliceSplitFirst {
                last: name.as_str() == "split_last",
            }),
            "is_char_boundary" if owner.is_str() => Std::Text(TextOp::IsCharBoundary),
            "len_utf8" if owner.is_char() => Std::Text(TextOp::CharLen { utf16: false }),
            "len_utf16" if owner.is_char() => Std::Text(TextOp::CharLen { utf16: true }),
            // Methods taking a pattern: only a string or a `char` one.
            "starts_with" | "ends_with" | "contains" | "replace" | "replacen" | "split" | "strip_prefix"
            | "strip_suffix" | "split_once" | "rsplit_once" | "find" | "rfind"
                if owner.is_str() && !self_ty.is_some_and(|p| self.is_string_like(p)) =>
            {
                return None;
            }
            // Splitting and searching by a `&str` or a `char` (ADR 0150); trimming by
            // a closure or a function too.
            "splitn" | "rsplitn" | "rsplit" | "split_terminator" | "match_indices" | "matches" | "split_inclusive"
            | "rsplit_terminator" | "rmatch_indices" | "rmatches"
                if owner.is_str() && !self_ty.is_some_and(|p| self.is_string_like(p)) =>
            {
                return None;
            }
            "splitn" if owner.is_str() => Std::Text(TextOp::SplitN),
            "rsplitn" if owner.is_str() => Std::Text(TextOp::Rsplit(true)),
            "rsplit" if owner.is_str() => Std::Text(TextOp::Rsplit(false)),
            "split_terminator" if owner.is_str() => Std::Text(TextOp::SplitTerminator),
            "split_inclusive" if owner.is_str() => Std::Text(TextOp::SplitInclusive),
            "rsplit_terminator" if owner.is_str() => Std::Text(TextOp::RsplitTerminator),
            "rmatch_indices" if owner.is_str() => Std::Text(TextOp::RmatchIndices),
            "rmatches" if owner.is_str() => Std::Text(TextOp::Rmatches),
            "split_at" if owner.is_str() => Std::Text(TextOp::SplitAt),
            "match_indices" if owner.is_str() => Std::Text(TextOp::MatchIndices),
            "matches" if owner.is_str() => Std::Text(TextOp::Matches),
            "trim_matches" | "trim_start_matches" | "trim_end_matches"
                if owner.is_str() && !self_ty.is_some_and(|p| self.is_string_like(p) || self.is_char_predicate(p)) =>
            {
                return None;
            }
            "trim_matches" if owner.is_str() => Std::Text(TextOp::TrimMatches { start: true, end: true }),
            // `trim_left` and `trim_right`, deprecated, are `trim_start` and
            // `trim_end`, and theirs of a pattern (ADR 0322).
            "trim_left_matches" | "trim_right_matches"
                if owner.is_str() && !self_ty.is_some_and(|p| self.is_string_like(p) || self.is_char_predicate(p)) =>
            {
                return None;
            }
            "trim_left_matches" if owner.is_str() => Std::Text(TextOp::TrimMatches {
                start: true,
                end: false,
            }),
            "trim_right_matches" if owner.is_str() => Std::Text(TextOp::TrimMatches {
                start: false,
                end: true,
            }),
            "trim_left" if owner.is_str() => Std::Trim {
                start: true,
                end: false,
            },
            "trim_right" if owner.is_str() => Std::Trim {
                start: false,
                end: true,
            },
            "trim_start_matches" if owner.is_str() => Std::Text(TextOp::TrimMatches {
                start: true,
                end: false,
            }),
            "trim_end_matches" if owner.is_str() => Std::Text(TextOp::TrimMatches {
                start: false,
                end: true,
            }),
            "find" if owner.is_str() => Std::Text(TextOp::Find(false)),
            "rfind" if owner.is_str() => Std::Text(TextOp::Find(true)),
            "starts_with" if owner.is_str() => Std::Method("startsWith"),
            "ends_with" if owner.is_str() => Std::Method("endsWith"),
            "contains" if owner.is_str() => Std::Method("includes"),
            "replace" if owner.is_str() => Std::Method("replaceAll"),
            // `replacen(p, r, 1)`: JS's `replace`, of the first (ADR 0034).
            "replacen" if owner.is_str() => Std::Method("replace"),
            "split" if owner.is_str() => Std::Method("split"),
            "strip_prefix" if owner.is_str() => Std::StripPrefix,
            "strip_suffix" if owner.is_str() => Std::StripSuffix,
            "strip_circumfix" if owner.is_str() && args.types().all(|p| self.is_string_like(p)) => Std::StripCircumfix,
            "split_once" if owner.is_str() => Std::SplitOnce,
            "rsplit_once" if owner.is_str() => Std::RsplitOnce,
            "to_uppercase" if owner.is_str() => Std::Method("toUpperCase"),
            "to_lowercase" if owner.is_str() => Std::Method("toLowerCase"),
            "trim_start" if owner.is_str() => Std::Trim {
                start: true,
                end: false,
            },
            "trim_end" if owner.is_str() => Std::Trim {
                start: false,
                end: true,
            },
            "repeat" if owner.is_str() => Std::Method("repeat"),
            // Of text, JS's `join`; of lists, their items with the separator's
            // between (ADR 0322). `connect`, deprecated, is `join`.
            "join" | "connect"
                if owner.is_slice() && self_ty.is_some_and(|item| self.is_string_like(item.peel_refs())) =>
            {
                Std::Method("join")
            }
            "join" | "connect" if owner.is_slice() => Std::JoinItems,
            "push_str" | "push" if string => Std::PushStr,
            "is_empty" if adt(StdItem::Vec) || owner.is_slice() || owner.is_str() || string => Std::IsEmpty,
            "lock" if self.stream(owner).is_some() => Std::Stream(StreamOp::Nothing),
            "contains" if range.is_some() => Std::Range(RangeOp::Contains),
            "start" if range == Some(RangeKind::Inclusive) => Std::Range(RangeOp::Bound("start")),
            "end" if range == Some(RangeKind::Inclusive) => Std::Range(RangeOp::Bound("end")),
            "into_inner" if range == Some(RangeKind::Inclusive) => Std::Range(RangeOp::IntoInner),
            "is_empty" if matches!(range, Some(RangeKind::Exclusive | RangeKind::Inclusive)) => {
                Std::Range(RangeOp::IsEmpty)
            }
            "as_ref" | "as_mut" if option => Std::Pointee,
            "get_or_insert" if option => Std::OptionPlace(OptionPlaceOp::GetOrInsert),
            "get_or_insert_with" if option => Std::OptionPlace(OptionPlaceOp::GetOrInsertWith),
            "get_or_insert_default" if option => Std::OptionPlace(OptionPlaceOp::GetOrInsertDefault),
            "insert" if option => Std::OptionPlace(OptionPlaceOp::Insert),
            "take_if" if option => Std::OptionPlace(OptionPlaceOp::TakeIf),
            // A `String`'s `&str` is the string, a `Vec`'s slice the array, a
            // box's or a reference's what it points at (ADRs 0211, 0326): of an
            // `Option` of one, it's the option; of a `Result`, the `Result`.
            // Another's `Deref` is refused.
            "as_deref" | "as_deref_mut"
                if (option || result)
                    && self_ty.is_some_and(|t| {
                        self.is_lang_adt(t, LangItem::String)
                            || self.is_std_type(t, StdItem::Vec)
                            || t.is_box()
                            || t.is_ref()
                    }) =>
            {
                if option {
                    Std::Pointee
                } else {
                    Std::Same
                }
            }
            "as_deref" if option || result => Std::Comb(Comb::AsDeref),
            // A reference is the value (ADR 0023): what's in the `Result` is.
            "as_ref" if result => Std::Same,
            "to_ascii_lowercase" if owner.is_str() => Std::AsciiCase { upper: false },
            "to_ascii_uppercase" if owner.is_str() => Std::AsciiCase { upper: true },
            "eq_ignore_ascii_case" if owner.is_str() || owner.is_char() => Std::AsciiEq,
            "append" if adt(StdItem::Vec) || adt(StdItem::VecDeque) || adt(StdItem::LinkedList) => Std::Append,
            "is_some" if option => Std::IsSome,
            "unwrap_unchecked" if option => Std::UnwrapUnchecked,
            "take" if option => Std::OptionTake,
            "replace" if option => Std::OptionReplace,
            "flatten" if option => Std::OptionFlatten,
            "iter" if option => Std::OptionIter,
            "iter_mut" if option => Std::OptionIterMut,
            "copied" | "cloned" if option => Std::OptionCloned,
            "is_none" if option => Std::IsNone,
            "unwrap_or" if option => Std::UnwrapOr,
            "map" if option => Std::OptionMap,
            // A thread-local (ADR 0037) is its `Cell` or `RefCell`: `{ value }`.
            "with" if local_key => Std::LocalWith,
            "get" if local_key => Std::CellGet,
            "set" if local_key => Std::CellSet,
            "with_borrow" | "with_borrow_mut" if local_key => Std::LocalBorrow {
                mutable: name.as_str() == "with_borrow_mut",
            },
            "then" if ordering => Std::Then,
            "then_with" if ordering => Std::ThenWith,
            "reverse" if ordering => Std::Reverse,
            "chars" if owner.is_str() => Std::Chars,
            "to_vec" if owner.is_slice() => Std::ToVec,
            "fill" if owner.is_slice() => Std::Slice(SliceOp::Fill),
            "fill_with" if owner.is_slice() => Std::Slice(SliceOp::FillWith),
            "copy_from_slice" if owner.is_slice() => Std::Slice(SliceOp::CopyFromSlice),
            "clone_from_slice" if owner.is_slice() => Std::Slice(SliceOp::CloneFromSlice),
            "swap_with_slice" if owner.is_slice() => Std::Slice(SliceOp::SwapWithSlice),
            "copy_within" if owner.is_slice() => Std::Slice(SliceOp::CopyWithin),
            "is_sorted" if owner.is_slice() => Std::Slice(SliceOp::IsSorted),
            "is_sorted_by" if owner.is_slice() => Std::Slice(SliceOp::IsSortedBy),
            "is_sorted_by_key" if owner.is_slice() => Std::Slice(SliceOp::IsSortedByKey),
            "partition_point" if owner.is_slice() => Std::Slice(SliceOp::PartitionPoint),
            "chunks_exact" | "chunks_exact_mut" if owner.is_slice() => Std::Slice(SliceOp::ChunksExact {
                mutable: name.as_str().ends_with("_mut"),
            }),
            "rchunks" | "rchunks_mut" | "rchunks_exact" | "rchunks_exact_mut" if owner.is_slice() => {
                Std::Slice(SliceOp::Rchunks {
                    exact: name.as_str().contains("exact"),
                    mutable: name.as_str().ends_with("_mut"),
                })
            }
            "chunks_mut" if owner.is_slice() => Std::Slice(SliceOp::ChunksMut),
            "remainder" | "into_remainder" if self.is_exact_chunks(owner) => Std::Slice(SliceOp::Remainder),
            "first_chunk" | "first_chunk_mut" | "last_chunk" | "last_chunk_mut" if owner.is_slice() => {
                Std::Slice(SliceOp::Chunk {
                    last: name.as_str().starts_with("last"),
                    mutable: name.as_str().ends_with("_mut"),
                })
            }
            "split_first_chunk" | "split_first_chunk_mut" | "split_last_chunk" | "split_last_chunk_mut"
                if owner.is_slice() =>
            {
                Std::Slice(SliceOp::SplitChunk {
                    last: name.as_str().starts_with("split_last"),
                    mutable: name.as_str().ends_with("_mut"),
                })
            }
            "split"
            | "splitn"
            | "rsplit"
            | "rsplitn"
            | "split_inclusive"
            | "split_mut"
            | "splitn_mut"
            | "rsplit_mut"
            | "rsplitn_mut"
            | "split_inclusive_mut"
                if owner.is_slice() =>
            {
                let base = name.as_str().trim_end_matches("_mut");
                Std::Slice(SliceOp::SplitBy {
                    limited: base.ends_with('n'),
                    inclusive: base == "split_inclusive",
                    back: base.starts_with('r'),
                    mutable: base != name.as_str(),
                })
            }
            "chunk_by" | "chunk_by_mut" if owner.is_slice() => Std::Slice(SliceOp::ChunkBy {
                mutable: name.as_str().ends_with("_mut"),
            }),
            "split_at_mut" | "split_at_mut_unchecked" | "split_at_mut_checked" if owner.is_slice() => {
                Std::Slice(SliceOp::SplitAtMut {
                    checked: name.as_str() == "split_at_mut_checked",
                })
            }
            "split_at_unchecked" if owner.is_slice() => Std::Text(TextOp::SliceSplitAt { checked: false }),
            "split_off"
            | "split_off_mut"
            | "split_off_first"
            | "split_off_first_mut"
            | "split_off_last"
            | "split_off_last_mut"
                if owner.is_slice() =>
            {
                let base = name.as_str().trim_end_matches("_mut");
                Std::Slice(SliceOp::SplitOff {
                    end: (base != "split_off").then_some(base == "split_off_last"),
                    mutable: base != name.as_str(),
                })
            }
            // A slice of `MaybeUninit`s is the array of what they hold (ADR 0332).
            "assume_init_ref" | "assume_init_mut" if owner.is_slice() => Std::Same,
            "write_copy_of_slice" | "write_clone_of_slice" if owner.is_slice() => Std::Slice(SliceOp::UninitWrite {
                clone: name.as_str() == "write_clone_of_slice",
            }),
            "assume_init_drop" if owner.is_slice() => Std::Slice(SliceOp::UninitDrop),
            // A byte slice's text: escaped, or its runs of UTF-8 (ADR 0341).
            "escape_ascii" if owner.is_slice() => Std::Text(TextOp::EscapeAscii),
            "utf8_chunks" if owner.is_slice() => Std::Text(TextOp::Utf8Chunks),
            "get_disjoint_mut" | "get_disjoint_unchecked_mut" if owner.is_slice() => {
                Std::Slice(SliceOp::GetDisjointMut {
                    unchecked: name.as_str() == "get_disjoint_unchecked_mut",
                })
            }
            // A slice as arrays of `N`, and arrays as one slice (ADR 0337).
            "as_array" | "as_mut_array" if owner.is_slice() => Std::Slice(SliceOp::AsArray),
            "array_windows" if owner.is_slice() => Std::Slice(SliceOp::ArrayWindows),
            "as_chunks"
            | "as_rchunks"
            | "as_chunks_mut"
            | "as_rchunks_mut"
            | "as_chunks_unchecked"
            | "as_chunks_unchecked_mut"
                if owner.is_slice() =>
            {
                Std::Slice(SliceOp::AsChunks {
                    back: name.as_str().starts_with("as_r"),
                    mutable: name.as_str().ends_with("_mut"),
                    unchecked: name.as_str().contains("unchecked"),
                })
            }
            "as_flattened" | "as_flattened_mut" if owner.is_slice() => Std::Slice(SliceOp::Flattened {
                mutable: name.as_str().ends_with("_mut"),
            }),
            "split_first_mut" | "split_last_mut" if owner.is_slice() => Std::Slice(SliceOp::SplitEndMut {
                last: name.as_str() == "split_last_mut",
            }),
            "get_mut" | "get_unchecked_mut"
                if owner.is_slice() && args.types().nth(1).is_some_and(|r| self.range_kind(r).is_some()) =>
            {
                Std::Slice(SliceOp::View {
                    checked: name.as_str() == "get_mut",
                })
            }
            "sort_by_cached_key" if owner.is_slice() => Std::Slice(SliceOp::SortByCachedKey),
            "repeat" if owner.is_slice() => Std::Slice(SliceOp::Repeat),
            "to_ascii_uppercase" | "to_ascii_lowercase" | "make_ascii_uppercase" | "make_ascii_lowercase"
                if owner.is_slice() =>
            {
                Std::Slice(SliceOp::AsciiCase {
                    upper: name.as_str().ends_with("uppercase"),
                    in_place: name.as_str().starts_with("make"),
                })
            }
            "is_ascii" if owner.is_slice() => Std::Slice(SliceOp::IsAscii),
            "trim_ascii" if owner.is_slice() => Std::Slice(SliceOp::TrimAscii { start: true, end: true }),
            "trim_ascii_start" if owner.is_slice() => Std::Slice(SliceOp::TrimAscii {
                start: true,
                end: false,
            }),
            "trim_ascii_end" if owner.is_slice() => Std::Slice(SliceOp::TrimAscii {
                start: false,
                end: true,
            }),
            // An unstable sort is JS's stable one only where items that compare
            // equal can't be told apart; else std's own (ADR 0342).
            "sort" if owner.is_slice() => Std::Sort,
            "sort_unstable" if owner.is_slice() && self_ty.is_some_and(|t| self.ties_unseen(t)) => Std::Sort,
            "sort_by" if owner.is_slice() => Std::SortBy,
            "sort_by_key" if owner.is_slice() => Std::SortByKey,
            n if owner.is_slice()
                && let Some(op) = unstable_sort(n) =>
            {
                Std::Slice(op)
            }
            "reverse" if owner.is_slice() => Std::Method("reverse"),
            // An array's `map` is JS's: a new array of what `f` makes of each.
            "map" if owner.is_array() => Std::Method("map"),
            // `v[0]` and `v.at(-1)` are `undefined` when `v` is empty: `None`.
            // A `_mut` one's item, of numbers or strings, is a handle on it
            // (ADR 0152).
            "first" | "first_mut" if owner.is_slice() => Std::First,
            "get" | "get_mut" if (owner.is_slice() && args.types().nth(1).is_some_and(|i| i.is_usize())) || deque => {
                Std::SliceGet
            }
            // Unchecked, the item, as JS reads one (ADR 0294).
            "get_unchecked" | "get_unchecked_mut"
                if owner.is_slice() && args.types().nth(1).is_some_and(|i| i.is_usize()) =>
            {
                Std::SliceGet
            }
            "last" | "last_mut" if owner.is_slice() => Std::SliceLast,
            // `includes` compares strings and numbers by value, as `==` does,
            // but objects by identity: only for those.
            // A `&mut` to one is a cell, an object (ADR 0099): not by identity.
            // Of items compared by their own `==` where it isn't JS's `===`
            // (ADR 0336).
            "starts_with" | "ends_with" if owner.is_slice() => Std::Text(TextOp::SliceStartsWith {
                end: name.as_str() == "ends_with",
            }),
            "strip_prefix" | "strip_suffix" | "strip_circumfix" if owner.is_slice() => Std::Slice(SliceOp::Strip {
                suffix: name.as_str() == "strip_suffix",
                circumfix: name.as_str() == "strip_circumfix",
            }),
            // Only `[u8]` has it.
            "eq_ignore_ascii_case" if owner.is_slice() => Std::Text(TextOp::BytesAsciiEq),
            "is_ok" if result => Std::IsOk(true),
            "is_err" if result => Std::IsOk(false),
            "ok" if result => Std::ResultOk,
            // An unchecked one panics where std's would be undefined behavior.
            "unwrap" | "expect" | "unwrap_unchecked" if result => Std::UnwrapOk,
            "unwrap_err" | "expect_err" | "unwrap_err_unchecked" if result => Std::UnwrapErr,
            "unwrap_or" if result => Std::ResultOr,
            _ => return None,
        })
    }

    pub(super) fn is_string_like(&self, ty: Ty<'tcx>) -> bool {
        let ty = ty.peel_refs();
        ty.is_str() || ty.is_char() || self.is_lang_adt(ty, LangItem::String)
    }

    /// Whether `ty` is standard output, or its lock, `Some(false)`, or
    /// standard error, `Some(true)` (ADR 0132).
    pub(super) fn stream(&self, ty: Ty<'tcx>) -> Option<bool> {
        let ty::Adt(adt, _) = ty.kind() else { return None };
        match std_path(self.tcx, adt.did()).as_str() {
            "std::io::Stdout" | "std::io::StdoutLock" => Some(false),
            "std::io::Stderr" | "std::io::StderrLock" => Some(true),
            _ => None,
        }
    }

    /// The runtime's `io::Write` of `ty`, or of what a `&mut` to it is to,
    /// if std's is one rust-js knows (ADR 0392): a `Vec<u8>`'s, which adds
    /// to it, a standard stream's, which writes to it, or a `Box<dyn
    /// Write>`'s, which is the pair it holds and writes through it.
    pub(super) fn io_writer(&self, ty: Ty<'tcx>) -> Option<&'static str> {
        let ty = match ty.kind() {
            ty::Ref(_, inner, Mutability::Mut) => *inner,
            _ => ty,
        };
        match self.stream(ty) {
            Some(false) => return Some("$stdoutWriter"),
            Some(true) => return Some("$stderrWriter"),
            None => {}
        }
        if ty.boxed_ty().is_some_and(|inner| {
            matches!(inner.kind(), ty::Dynamic(traits, ..)
                if traits.principal_def_id().is_some_and(|id| is_std_def(self.tcx, id, StdItem::IoWrite)))
        }) {
            return Some("$dynWriter");
        }
        let bytes = matches!(ty.kind(), ty::Adt(_, args) if self.is_std_type(ty, StdItem::Vec)
            && matches!(args.type_at(0).kind(), ty::Uint(ty::UintTy::U8)));
        bytes.then_some("$vecWriter")
    }

    /// A write's result, `fmt::Result` or `io::Result<()>` (`is_write_result_type`).
    pub(super) fn is_write_result(&self, ty: Ty<'tcx>) -> bool {
        is_write_result_type(self.tcx, ty)
    }

    /// `io::Result<()>`, a write's result (`is_write_result_type`).
    pub(super) fn is_io_unit_result(&self, ty: Ty<'tcx>) -> bool {
        is_io_unit_result_type(self.tcx, ty)
    }

    /// `Layout`, `{ size, align }` (ADR 0417), `Some(true)`; or
    /// `LayoutError`, which says nothing, `Some(false)`.
    pub(super) fn layout(&self, ty: Ty<'tcx>) -> Option<bool> {
        let ty::Adt(adt, _) = ty.kind() else { return None };
        match std_path(self.tcx, adt.did()).as_str() {
            "std::alloc::Layout" => Some(true),
            "std::alloc::LayoutError" => Some(false),
            _ => None,
        }
    }

    /// `panic::Location`: `{ file, line, column }` (ADR 0427).
    pub(super) fn is_location(&self, ty: Ty<'tcx>) -> bool {
        matches!(ty.kind(), ty::Adt(location, _) if std_path(self.tcx, location.did()) == "std::panic::Location")
    }

    /// std's `iter::Empty<T>`: `[]`, as `iter::empty()` is (ADR 0128).
    fn is_iter_empty(&self, ty: Ty<'tcx>) -> bool {
        matches!(ty.kind(), ty::Adt(empty, _) if std_path(self.tcx, empty.did()) == "std::iter::Empty")
    }

    /// `io::Error`, a `$IoError` (ADR 0462).
    pub(super) fn is_io_error(&self, ty: Ty<'tcx>) -> bool {
        matches!(ty.kind(), ty::Adt(error, _) if std_path(self.tcx, error.did()) == "std::io::Error")
    }

    /// `io::ErrorKind`, its variant's name (ADR 0462).
    pub(super) fn is_io_error_kind(&self, ty: Ty<'tcx>) -> bool {
        matches!(ty.kind(), ty::Adt(kind, _) if std_path(self.tcx, kind.did()) == "std::io::ErrorKind")
    }

    /// Which of `io::Error`'s functions `def_id` is: its `new`, `other` and
    /// `kind`, and `From<ErrorKind>`'s, or `Into`'s of one.
    fn io_error_op(&self, def_id: DefId, args: ty::GenericArgsRef<'tcx>) -> Option<IoErrorOp> {
        let tcx = self.tcx;
        if let Some(imp) = tcx.inherent_impl_of_assoc(def_id)
            && self.is_io_error(tcx.type_of(imp).instantiate_identity().skip_normalization())
        {
            return match tcx.item_name(def_id).as_str() {
                "new" => Some(IoErrorOp::New),
                "other" => Some(IoErrorOp::Other),
                "kind" => Some(IoErrorOp::Kind),
                _ => None,
            };
        }
        let tr = tcx.trait_of_assoc(def_id)?;
        let mut types = args.types();
        let (first, second) = (types.next()?, types.next()?);
        let (to, from) = match () {
            _ if is_std_def(tcx, tr, StdItem::From) => (first, second),
            _ if is_std_def(tcx, tr, StdItem::Into) => (second, first),
            _ => return None,
        };
        (self.is_io_error(to) && self.is_io_error_kind(from)).then_some(IoErrorOp::FromKind)
    }

    /// Which of hashing's functions `def_id` is (ADR 0463): a std hasher's
    /// `new` and `new_with_keys`, its own `Hasher` methods, and a std type's
    /// `Hash`.
    fn hash_op(&self, def_id: DefId, args: ty::GenericArgsRef<'tcx>) -> Option<HashOp> {
        let tcx = self.tcx;
        match std_path(tcx, def_id).as_str() {
            "std::hash::DefaultHasher::new" => return Some(HashOp::New(StdHasher::Default)),
            "std::hash::SipHasher::new" => return Some(HashOp::New(StdHasher::Sip)),
            "std::hash::SipHasher::new_with_keys" => return Some(HashOp::NewWithKeys),
            _ => {}
        }
        let tr = tcx.trait_of_assoc(def_id)?;
        let self_ty = args.types().next()?;
        if is_hasher(tcx, tr) && self.std_hasher(self_ty).is_some() {
            return Some(HashOp::Own);
        }
        // A crate's hasher's provided method that its impl doesn't write,
        // `count.write_i32(n)`: its dictionary's, std's over its own.
        if is_hasher(tcx, tr) && self.left_to_std(def_id, self_ty) {
            return Some(HashOp::Provided);
        }
        if !is_std_def(tcx, tr, StdItem::Hash) {
            return None;
        }
        let std = self.hashed_as(self_ty).is_some();
        match tcx.item_name(def_id).as_str() {
            "hash" if std => Some(HashOp::Hash),
            // Of a crate's type too, whose impl leaves it to std: its
            // dictionary's, `$hashSlice`'s.
            "hash_slice" if std || self.left_to_std(def_id, self_ty) => Some(HashOp::HashSlice),
            _ => None,
        }
    }

    /// Whether `self_ty`'s impl of the trait of provided method `def_id`
    /// leaves it to std's: a crate's type's whose impl doesn't write it.
    fn left_to_std(&self, def_id: DefId, self_ty: Ty<'tcx>) -> bool {
        let tcx = self.tcx;
        let Some(tr) = tcx.trait_of_assoc(def_id) else {
            return false;
        };
        matches!(self_ty.kind(), ty::Adt(..))
            && matches!(
                tcx.codegen_select_candidate(
                    self.typing_env
                        .as_query_input(tcx.erase_and_anonymize_regions(ty::TraitRef::new(tcx, tr, [self_ty]))),
                ),
                Ok(ImplSource::UserDefined(imp))
                    if !tcx.impl_item_implementor_ids(imp.impl_def_id).contains_key(&def_id)
            )
    }

    /// std's `Hasher`, found among the traits, as it has no diagnostic item.
    pub(super) fn hasher_trait(&self) -> Option<DefId> {
        self.tcx
            .all_traits_including_private()
            .find(|&id| is_hasher(self.tcx, id))
    }

    /// Which of std's hashers `ty` is, a `$SipHasher` (ADR 0463).
    pub(super) fn std_hasher(&self, ty: Ty<'tcx>) -> Option<StdHasher> {
        let ty::Adt(adt, _) = ty.kind() else { return None };
        match std_path(self.tcx, adt.did()).as_str() {
            "std::hash::DefaultHasher" => Some(StdHasher::Default),
            "std::hash::SipHasher" => Some(StdHasher::Sip),
            _ => None,
        }
    }

    /// What std's `Hash` of a `ty` writes, where it's std's (ADR 0463):
    /// `None` of a crate's type, a type parameter, or one rust-js doesn't
    /// hash.
    pub(super) fn hashed_as(&self, ty: Ty<'tcx>) -> Option<Hashed<'tcx>> {
        Some(match *ty.kind() {
            ty::Int(_) | ty::Uint(_) => Hashed::Number(ty),
            // A `NonZero` is its number (ADR 0177), and an `Ordering` its
            // `i8`, `-1`, `0` or `1`, as its derived `Hash` writes it.
            ty::Adt(_, args) if is_non_zero_ty(ty) => Hashed::Number(args.type_at(0)),
            ty::Adt(..) if self.is_lang_adt(ty, LangItem::OrderingEnum) => Hashed::Number(self.tcx.types.i8),
            ty::Bool => Hashed::Bool,
            ty::Char => Hashed::Char,
            ty::Str => Hashed::Text,
            ty::Ref(_, pointee, _) => Hashed::Pointer(pointee),
            ty::Tuple(parts) => Hashed::Tuple(parts.to_vec()),
            ty::Slice(item) | ty::Array(item, _) => Hashed::Slice(item),
            ty::Adt(_, args) if ty.is_box() || self.is_rc(ty) => Hashed::Pointer(args.type_at(0)),
            ty::Adt(_, args) if self.is_std_type(ty, StdItem::Vec) => Hashed::Slice(args.type_at(0)),
            ty::Adt(_, args) if self.is_std_type(ty, StdItem::VecDeque) => Hashed::Items(args.type_at(0)),
            ty::Adt(_, args) if self.option_of(ty).is_some() => Hashed::Option(args.type_at(0)),
            ty::Adt(_, args) if self.is_std_type(ty, StdItem::Result) && !self.is_write_result(ty) => {
                Hashed::Result(args.type_at(0), args.type_at(1))
            }
            ty::Adt(..) if self.is_lang_adt(ty, LangItem::String) => Hashed::Text,
            ty::Adt(..) if self.is_std_type(ty, StdItem::PhantomData) => Hashed::Tuple(Vec::new()),
            _ => return None,
        })
    }

    /// Which of std's ranges `ty` is, if it's one (ADR 0129).
    pub(super) fn range_kind(&self, ty: Ty<'tcx>) -> Option<RangeKind> {
        let ty::Adt(adt, _) = ty.kind() else { return None };
        [
            (LangItem::Range, RangeKind::Exclusive),
            (LangItem::RangeInclusiveStruct, RangeKind::Inclusive),
            (LangItem::RangeFrom, RangeKind::From),
            (LangItem::RangeTo, RangeKind::To),
            (LangItem::RangeToInclusive, RangeKind::ToInclusive),
            (LangItem::RangeFull, RangeKind::Full),
        ]
        .into_iter()
        .find(|&(item, _)| self.tcx.is_lang_item(adt.did(), item))
        .map(|(_, kind)| kind)
    }

    /// Is `ty` std's `item`?
    pub(super) fn is_std_type(&self, ty: Ty<'tcx>, item: StdItem) -> bool {
        matches!(ty.kind(), ty::Adt(adt, _) if is_std_def(self.tcx, adt.did(), item))
    }

    pub(super) fn is_lang_adt(&self, ty: Ty<'tcx>, item: LangItem) -> bool {
        matches!(ty.kind(), ty::Adt(adt, _) if self.tcx.is_lang_item(adt.did(), item))
    }

    /// A `Box<[T; N]>` or a `Box<[T]>`: its array, in JS (ADR 0023).
    pub(super) fn is_boxed_items(&self, ty: Ty<'tcx>) -> bool {
        ty.boxed_ty().is_some_and(|inner| inner.is_array() || inner.is_slice())
    }

    /// What `extend` of a std collection is, of `ty`: a `String`'s, a
    /// `push_str` of its items' text (ADR 0445); a `Vec`'s (ADR 0062); a map's
    /// or a set's, each pair set, or item added (ADR 0325). None of another
    /// type's, whose impl's is its own.
    pub(super) fn std_extend(&self, ty: Ty<'tcx>) -> Option<Std> {
        if self.is_lang_adt(ty, LangItem::String) {
            Some(Std::PushStr)
        } else if self.is_vec_like(ty) {
            Some(Std::Comb(Comb::Extend))
        } else if self.is_map(ty) && !self.is_json_map(ty) {
            Some(Std::Map(MapOp::Extend))
        } else {
            None
        }
    }

    /// `a += b` of a number whose `+` is its type's own, a `Duration`'s or
    /// a clock's, not its number's: `a = a + b`, of that `+`. The operator's
    /// method, its arguments, and what it is (ADRs 0188, 0447).
    pub(super) fn assigned_by(
        &self,
        assign_trait: DefId,
        ty: Ty<'tcx>,
        rhs: Ty<'tcx>,
    ) -> Option<(DefId, ty::GenericArgsRef<'tcx>, Std)> {
        let tcx = self.tcx;
        let op = assign_operator(tcx, assign_trait)?;
        let (item, name) = [
            (BinOp::Add, LangItem::Add, "add"),
            (BinOp::Sub, LangItem::Sub, "sub"),
            (BinOp::Mul, LangItem::Mul, "mul"),
            (BinOp::Div, LangItem::Div, "div"),
            (BinOp::Rem, LangItem::Rem, "rem"),
        ]
        .into_iter()
        .find_map(|(o, item, name)| (o == op).then_some((item, name)))?;
        let method = trait_method(tcx, tcx.lang_items().get(item)?, name);
        let args = tcx.mk_args(&[ty.into(), rhs.into()]);
        Some((method, args, self.classify(method, args)?))
    }

    /// A clock's operator, of its trait `trait_` and its arguments (ADR
    /// 0447): `t + d` and `t - d` of a `Duration`, and an `Instant`'s
    /// `a - b` of another. `t += d` is std's `*t = *t + d`, as a
    /// `Duration`'s is (ADR 0188); its others aren't its number's.
    fn clock_operator(&self, trait_: DefId, args: ty::GenericArgsRef<'tcx>, clock: ClockType) -> Option<Std> {
        let tcx = self.tcx;
        let instant = clock == ClockType::Instant;
        let rhs = args.types().nth(1)?;
        let shift = |sub: bool| {
            Some(Std::Clock {
                op: ClockOp::Shift { sub },
                instant,
            })
        };
        match (is_duration_ty(rhs), clock_type_of(rhs)) {
            (true, _) if tcx.is_lang_item(trait_, LangItem::Add) => shift(false),
            (true, _) if tcx.is_lang_item(trait_, LangItem::Sub) => shift(true),
            (false, Some(ClockType::Instant)) if instant && tcx.is_lang_item(trait_, LangItem::Sub) => {
                Some(Std::Clock {
                    op: ClockOp::Since { checked: false },
                    instant,
                })
            }
            _ => None,
        }
    }

    /// The call a std function is, which rust-js lowers as that call:
    /// `fmt::write(w, args)` is `w.write_fmt(args)`, of the `dyn fmt::Write`
    /// it takes, anyhow's (ADR 0446). Another is itself.
    pub(super) fn call_it_is(
        &self,
        def_id: DefId,
        generic_args: ty::GenericArgsRef<'tcx>,
    ) -> (DefId, ty::GenericArgsRef<'tcx>) {
        let tcx = self.tcx;
        if tcx.crate_name(def_id.krate) == sym::core
            && std_path(tcx, def_id).ends_with("fmt::write")
            && let sig = tcx
                .instantiate_bound_regions_with_erased(tcx.fn_sig(def_id).instantiate_identity().skip_normalization())
            && let Some(&writer) = sig.inputs().first()
        {
            let write_fmt = trait_method(tcx, std_item(tcx, StdItem::FmtWrite), "write_fmt");
            return (write_fmt, tcx.mk_args(&[writer.peel_refs().into()]));
        }
        (def_id, generic_args)
    }

    pub(super) fn is_vec_like(&self, ty: Ty<'tcx>) -> bool {
        self.is_std_type(ty, StdItem::Vec)
            || [StdItem::VecDeque, StdItem::BinaryHeap, StdItem::LinkedList]
                .into_iter()
                .any(|item| self.is_std_type(ty, item))
    }

    /// `std::str::Chars`, what a string's `chars()` gives.
    pub(super) fn is_str_chars(&self, ty: Ty<'tcx>) -> bool {
        matches!(ty.kind(), ty::Adt(adt, _) if self.tcx.crate_name(adt.did().krate) == sym::core
            && self.tcx.item_name(adt.did()).as_str() == "Chars")
    }

    /// A string's `bytes()`, an array of its UTF-8 bytes (ADR 0034).
    pub(super) fn is_str_bytes(&self, ty: Ty<'tcx>) -> bool {
        matches!(ty.kind(), ty::Adt(adt, _) if self.tcx.crate_name(adt.did().krate) == sym::core
            && self.tcx.item_name(adt.did()).as_str() == "Bytes")
    }

    /// `js::JsError`, what a JS function threw: the builtins crate's.
    pub(super) fn is_js_error(&self, ty: Ty<'tcx>) -> bool {
        matches!(ty.kind(), ty::Adt(adt, _) if self.tcx.crate_name(adt.did().krate).as_str() == "js"
            && self.tcx.item_name(adt.did()).as_str() == "JsError")
    }

    pub(super) fn is_js_object(&self, ty: Ty<'tcx>) -> bool {
        let ty::Adt(adt, args) = ty.kind() else { return false };
        if !adt.is_struct() {
            return false;
        }
        // `PhantomData<JsObject>`, then only more markers, for a generic one
        // like `Promise<T>`.
        let mut fields = adt.non_enum_variant().fields.iter().map(|f| self.field_ty(f, args));
        let first = fields.next();
        first.is_some_and(|field| {
            matches!(field.kind(), ty::Adt(marker, marked) if marker.is_phantom_data()
            && marked.types().next().is_some_and(|t| matches!(t.kind(), ty::Foreign(_)) || self.is_js_object_itself(t)))
        }) && fields.all(|field| matches!(field.kind(), ty::Adt(marker, _) if marker.is_phantom_data()))
    }

    /// The js crate's `JsObject`, `#[rust_js::js_object]`, a struct no one makes,
    /// where it was an extern type, a nightly feature (ADR 0111): a struct of a
    /// `PhantomData` of it is a JS object, as one of an extern type still is.
    /// A program never has a `JsObject` itself, only a reference to one.
    fn is_js_object_itself(&self, ty: Ty<'tcx>) -> bool {
        matches!(ty.kind(), ty::Adt(adt, _) if self.tcx
            .get_attrs_by_path(adt.did(), &[Symbol::intern("rust_js"), Symbol::intern("js_object")])
            .next()
            .is_some())
    }

    pub(super) fn is_map(&self, ty: Ty<'tcx>) -> bool {
        let ty = ty.peel_refs();
        [StdItem::HashMap, StdItem::HashSet, StdItem::BTreeMap, StdItem::BTreeSet]
            .into_iter()
            .any(|item| self.is_std_type(ty, item))
            || self.is_json_map(ty)
    }

    /// A pattern that's a predicate of a `char`: a closure, a function, or a
    /// set of `char`s, an array or a slice of them (ADR 0157).
    pub(super) fn is_char_predicate(&self, pattern: Ty<'tcx>) -> bool {
        match pattern.peel_refs().kind() {
            ty::Closure(..) | ty::FnDef(..) | ty::FnPtr(..) => true,
            ty::Array(item, _) | ty::Slice(item) => item.is_char(),
            _ => false,
        }
    }

    pub(super) fn is_set(&self, ty: Ty<'tcx>) -> bool {
        let ty = ty.peel_refs();
        self.is_std_type(ty, StdItem::HashSet) || self.is_std_type(ty, StdItem::BTreeSet)
    }

    pub(super) fn is_peekable(&self, ty: Ty<'tcx>) -> bool {
        matches!(ty.peel_refs().kind(), ty::Adt(adt, _) if self.tcx.crate_name(adt.did().krate) == rustc_span::sym::core
            && self.tcx.item_name(adt.did()).as_str() == "Peekable")
    }

    /// A channel's end, `mpsc::Sender` or `mpsc::Receiver` (ADR 0142).
    pub(super) fn channel_end(&self, ty: Ty<'tcx>) -> Option<ChannelEnd> {
        let ty::Adt(adt, _) = ty.kind() else {
            return None;
        };
        match std_path(self.tcx, adt.did()).as_str() {
            "std::sync::mpsc::Sender" => Some(ChannelEnd::Sender),
            "std::sync::mpsc::Receiver" => Some(ChannelEnd::Receiver),
            _ => None,
        }
    }

    /// A channel's error, which std defines with its multi-producer channels.
    pub(super) fn channel_error(&self, ty: Ty<'tcx>) -> Option<ChannelError> {
        let ty::Adt(adt, _) = ty.kind() else {
            return None;
        };
        let path = std_path(self.tcx, adt.did());
        if !path.starts_with("std::sync::mpsc::") && !path.starts_with("std::sync::mpmc::") {
            return None;
        }
        match self.tcx.item_name(adt.did()).as_str() {
            "RecvError" => Some(ChannelError::Recv),
            "TryRecvError" => Some(ChannelError::TryRecv),
            "SendError" => Some(ChannelError::Send),
            _ => None,
        }
    }

    /// A `dyn Iterator`, boxed or lent: a JS iterator, whatever made it; of
    /// a `dyn DoubleEndedIterator`, one that knows its back (ADR 0429).
    pub(super) fn is_dyn_iter(&self, ty: Ty<'tcx>) -> bool {
        is_dyn_iter(self.tcx, ty)
    }

    /// The `fmt` trait a placeholder of `kind` calls, `{:x}`'s `LowerHex`,
    /// where it's one of `OTHER_FMT_TRAITS`.
    pub(super) fn other_fmt_trait(&self, kind: Std) -> Option<DefId> {
        let name = match kind {
            Std::FmtRadix(Radix::LowerHex) => "LowerHex",
            Std::FmtRadix(Radix::UpperHex) => "UpperHex",
            Std::FmtRadix(Radix::Octal) => "Octal",
            Std::FmtRadix(Radix::Binary) => "Binary",
            Std::FmtExp(false) => "LowerExp",
            Std::FmtExp(true) => "UpperExp",
            Std::FmtPointer => "Pointer",
            _ => return None,
        };
        self.tcx
            .all_traits_including_private()
            .find(|&id| is_other_fmt_trait(self.tcx, id) && self.tcx.item_name(id).as_str() == name)
    }

    /// Can no two `ty` items that `Ord` calls equal be told apart? Of
    /// numbers, `bool`s, `char`s and text, and tuples and arrays of them; not
    /// through a `&mut`, a cell, an object (ADR 0099).
    fn ties_unseen(&self, ty: Ty<'tcx>) -> bool {
        match ty.kind() {
            ty::Tuple(items) => items.iter().all(|item| self.ties_unseen(item)),
            ty::Array(item, _) => self.ties_unseen(*item),
            _ => {
                !ty.walk()
                    .any(|part| matches!(part.as_type().map(|p| *p.kind()), Some(ty::Ref(_, _, Mutability::Mut))))
                    && (self.is_string_like(ty) || Num::of(ty).is_some() || ty.is_bool() || ty.is_char())
            }
        }
    }

    /// core's `DoubleEndedIterator`, found among the traits, as it has no
    /// diagnostic item.
    pub(super) fn double_ended_iterator(&self) -> Option<DefId> {
        self.tcx
            .all_traits_including_private()
            .find(|&id| is_double_ended(self.tcx, id))
    }

    pub(super) fn exact_size_iterator(&self) -> Option<DefId> {
        self.tcx
            .all_traits_including_private()
            .find(|&id| is_iterator_extension(self.tcx, id) && self.tcx.item_name(id).as_str() == "ExactSizeIterator")
    }

    pub(super) fn is_user_iterator(&self, ty: ty::Ty<'tcx>) -> bool {
        let iterator = self.tcx.get_diagnostic_item(sym::Iterator).expect("std has `Iterator`");
        matches!(ty.peel_refs().kind(), ty::Adt(..)) && self.has_user_impl(iterator, ty.peel_refs())
    }

    pub(super) fn args_of(&self, trait_id: DefId, ty: Ty<'tcx>) -> ty::GenericArgsRef<'tcx> {
        let ty = self.tcx.erase_and_anonymize_regions(ty);
        self.tcx.mk_args_from_iter(std::iter::repeat_n(
            ty::GenericArg::from(ty),
            self.tcx.generics_of(trait_id).count(),
        ))
    }

    pub(super) fn has_user_impl(&self, trait_id: DefId, ty: Ty<'tcx>) -> bool {
        self.is_user_impl(ty::TraitRef::new_from_args(
            self.tcx,
            trait_id,
            self.args_of(trait_id, ty),
        ))
    }

    /// Whether the crate's impl of `trait_id` for `ty` writes the method
    /// `name` itself, not taking its trait's default.
    pub(super) fn user_impl_defines(&self, trait_id: DefId, ty: Ty<'tcx>, name: &str) -> bool {
        let tr = ty::TraitRef::new_from_args(self.tcx, trait_id, self.args_of(trait_id, ty));
        let tr = self.tcx.erase_and_anonymize_regions(tr);
        matches!(self.tcx.codegen_select_candidate(self.typing_env.as_query_input(tr)),
            Ok(ImplSource::UserDefined(imp)) if self.tcx.associated_items(imp.impl_def_id)
                .filter_by_name_unhygienic(Symbol::intern(name))
                .next()
                .is_some())
    }

    /// The crate's impl of serde's `Visitor` for `ty`, a hand-written
    /// `Deserialize`'s (ADR 0459).
    pub(super) fn serde_visitor_impl(&self, ty: Ty<'tcx>) -> Option<DefId> {
        let visitor = (self.tcx.all_traits_including_private()).find(|&id| serde_visitor(self.tcx, id))?;
        // `Visitor<'de>` of `ty`: its `Self`, and its lifetime erased.
        let args = ty::GenericArgs::for_item(self.tcx, visitor, |param, _| match param.kind {
            ty::GenericParamDefKind::Lifetime => self.tcx.lifetimes.re_erased.into(),
            _ => ty.into(),
        });
        let tr = ty::TraitRef::new_from_args(self.tcx, visitor, args);
        match self.tcx.codegen_select_candidate(self.typing_env.as_query_input(tr)) {
            Ok(ImplSource::UserDefined(imp)) if self.trait_impls.contains(&imp.impl_def_id) => Some(imp.impl_def_id),
            _ => None,
        }
    }

    pub(super) fn is_user_impl(&self, tr: ty::TraitRef<'tcx>) -> bool {
        let tr = self.tcx.erase_and_anonymize_regions(tr);
        matches!(self.tcx.codegen_select_candidate(self.typing_env.as_query_input(tr)),
            Ok(ImplSource::UserDefined(imp))
                if self.trait_impls.contains(&imp.impl_def_id) || self.foreign.has_impl(imp.impl_def_id))
    }

    pub(super) fn json_type(&self, ty: Ty<'tcx>) -> Option<Json> {
        let ty::Adt(adt, _) = ty.peel_refs().kind() else {
            return None;
        };
        if self.tcx.crate_name(adt.did().krate).as_str() != "serde_json" {
            return None;
        }
        Some(match self.tcx.item_name(adt.did()).as_str() {
            "Value" => Json::Value,
            "Number" => Json::Number,
            "Map" => Json::Map,
            _ => return None,
        })
    }

    pub(super) fn is_json_map(&self, ty: Ty<'tcx>) -> bool {
        self.json_type(ty) == Some(Json::Map)
    }
}

/// A call of serde's data model a hand-written `Serialize` or `Deserialize`
/// makes, over the JSON writer and reader (ADR 0459).
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) enum SerdeOp {
    /// `s.serialize_bool(b)` and the like: a value the writer writes itself.
    Write(SerdeScalar),
    /// `serialize_some(&v)` and `serialize_newtype_struct(name, &v)`: `v`, as
    /// it is, its argument at `at`.
    WriteValue {
        at: usize,
    },
    /// `serialize_unit_variant(name, index, variant)`: its name.
    UnitVariant,
    /// `serialize_newtype_variant(name, index, variant, &v)`: `{ variant: v }`.
    NewtypeVariant,
    /// `collect_str(&v)`: what `v` shows, as a string.
    CollectStr,
    /// `collect_seq(items)` and `collect_map(entries)`: each written in turn,
    /// in an array or an object.
    CollectSeq,
    CollectMap,
    /// `serialize_seq(len)` and the rest: the writer, begun.
    Begin(SerdeCompound),
    /// `serialize_element(&v)`, of a sequence or a tuple: its next.
    Element,
    /// `serialize_field(key, &v)` of a struct: its key, then it.
    Field,
    /// `skip_field(key)`: nothing.
    SkipField,
    /// `serialize_entry(&k, &v)`, `serialize_key(&k)` and `serialize_value(&v)`
    /// of a map.
    Entry,
    Key,
    Value,
    /// `end()`: the compound closed.
    End(SerdeCompound),
    /// `v.serialize(s)` of a `Serialize`: its writer's.
    Serialize,
    /// `T::deserialize(d)` of a `Deserialize`: its reader's, a `Result`.
    Deserialize,
    /// `d.deserialize_any(v)` and the other hints, of a hand-written
    /// visitor: what's there, given to the `visit_*` of its kind the hint
    /// takes.
    Visit(SerdeHint),
    /// `seq.next_element::<T>()`, and a map's `next_key`, `next_value` and
    /// `next_entry`: what's next, a `Result` of an `Option` but a value's.
    Next(SerdeNext),
    /// `S::Error::custom(msg)` and `D::Error::custom(msg)`: an error of what
    /// `msg` shows.
    Custom,
    /// `E::invalid_type(unexpected, &self)` and serde's other errors of a
    /// deserializer, its messages.
    DeError(SerdeError),
}

/// serde's `de::Error`s but `custom`, each its message.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) enum SerdeError {
    InvalidType,
    InvalidValue,
    InvalidLength,
    UnknownVariant,
    UnknownField,
    MissingField,
    DuplicateField,
}

/// What a writer writes itself, of serde's scalars.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) enum SerdeScalar {
    Bool,
    Int,
    Float {
        single: bool,
    },
    Char,
    Str,
    /// `()`, `None` and a unit struct: `null`.
    Null,
}

/// What a `deserialize_*` hint takes, of the kinds a JSON value is: any
/// of them, a `bool`, a number, a string, or `null`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) enum SerdeHint {
    Any,
    Bool,
    Number,
    Str,
    Unit,
    /// A sequence, `deserialize_seq`'s and a tuple's.
    Seq,
    Map,
    /// `deserialize_struct`'s, a map or a sequence of its fields.
    Struct,
    /// `deserialize_option`'s: `null`, or anything else, given to read.
    Option,
}

/// What a `SeqAccess` or a `MapAccess` reads next.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) enum SerdeNext {
    Element,
    Key,
    Value,
    Entry,
}

/// A hand-written visitor's method, of those serde_json calls: what it's
/// expecting, and a `visit_*` of each kind of JSON value.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) enum VisitorMethod {
    Expecting,
    Unit,
    Bool,
    U64,
    I64,
    F64,
    Str,
    BorrowedStr,
    Seq,
    Map,
    /// `visit_none` and `visit_some`, of `deserialize_option`.
    None,
    Some,
}

/// The methods of serde's `Visitor` the impl `imp` writes, of those
/// serde_json calls, each with its function (ADR 0459).
pub(super) fn visitor_methods(tcx: TyCtxt<'_>, imp: DefId) -> Vec<(VisitorMethod, DefId)> {
    (tcx.associated_items(imp).in_definition_order())
        .filter_map(|item| {
            let method = match item.name().as_str() {
                "expecting" => VisitorMethod::Expecting,
                "visit_unit" => VisitorMethod::Unit,
                "visit_bool" => VisitorMethod::Bool,
                "visit_u64" => VisitorMethod::U64,
                "visit_i64" => VisitorMethod::I64,
                "visit_f64" => VisitorMethod::F64,
                "visit_str" => VisitorMethod::Str,
                "visit_borrowed_str" => VisitorMethod::BorrowedStr,
                "visit_seq" => VisitorMethod::Seq,
                "visit_map" => VisitorMethod::Map,
                "visit_none" => VisitorMethod::None,
                "visit_some" => VisitorMethod::Some,
                _ => return None,
            };
            Some((method, item.def_id))
        })
        .collect()
}

/// What a `serialize_*` begins: a sequence, a tuple, a tuple struct, a map
/// or a struct.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) enum SerdeCompound {
    Seq,
    Tuple,
    TupleStruct,
    Map,
    Struct,
    /// A variant's, in an object of its name.
    TupleVariant,
    StructVariant,
}

/// Is `ty` a `dyn` of serde's `de::Expected`, what a deserializer's error
/// says it expected, which rust-js holds as its text (ADR 0459)?
pub(super) fn is_dyn_expected(tcx: TyCtxt<'_>, ty: Ty<'_>) -> bool {
    matches!(ty.peel_refs().kind(), ty::Dynamic(traits, ..)
        if traits.principal_def_id().is_some_and(|id| matches!(tcx.crate_name(id.krate).as_str(), "serde" | "serde_core")
            && tcx.item_name(id).as_str() == "Expected"))
}

/// Is `id` serde's `de::Visitor`?
pub(super) fn serde_visitor(tcx: TyCtxt<'_>, id: DefId) -> bool {
    matches!(tcx.crate_name(id.krate).as_str(), "serde" | "serde_core")
        && tcx.def_kind(id) == DefKind::Trait
        && tcx.item_name(id).as_str() == "Visitor"
}

/// `def_id` as a call of serde's data model a hand-written impl makes (ADR
/// 0459), or `None`.
pub(super) fn serde_op(tcx: TyCtxt<'_>, def_id: DefId) -> Option<SerdeOp> {
    let trait_id = tcx.trait_of_assoc(def_id)?;
    if !matches!(tcx.crate_name(trait_id.krate).as_str(), "serde" | "serde_core") {
        return None;
    }
    let method = tcx.item_name(def_id);
    let compound = |name: &str| match name {
        "SerializeSeq" => Some(SerdeCompound::Seq),
        "SerializeTuple" => Some(SerdeCompound::Tuple),
        "SerializeTupleStruct" => Some(SerdeCompound::TupleStruct),
        "SerializeMap" => Some(SerdeCompound::Map),
        "SerializeStruct" => Some(SerdeCompound::Struct),
        "SerializeTupleVariant" => Some(SerdeCompound::TupleVariant),
        "SerializeStructVariant" => Some(SerdeCompound::StructVariant),
        _ => None,
    };
    let trait_name = tcx.item_name(trait_id);
    Some(match (trait_name.as_str(), method.as_str()) {
        ("Serialize", "serialize") => SerdeOp::Serialize,
        ("Deserialize", "deserialize") => SerdeOp::Deserialize,
        ("Deserializer", name) => SerdeOp::Visit(match name {
            "deserialize_any" => SerdeHint::Any,
            "deserialize_bool" => SerdeHint::Bool,
            "deserialize_i8" | "deserialize_i16" | "deserialize_i32" | "deserialize_i64" | "deserialize_u8"
            | "deserialize_u16" | "deserialize_u32" | "deserialize_u64" | "deserialize_f32" | "deserialize_f64" => {
                SerdeHint::Number
            }
            "deserialize_str" | "deserialize_string" | "deserialize_identifier" => SerdeHint::Str,
            "deserialize_unit" => SerdeHint::Unit,
            "deserialize_seq" | "deserialize_tuple" | "deserialize_tuple_struct" => SerdeHint::Seq,
            "deserialize_map" => SerdeHint::Map,
            "deserialize_struct" => SerdeHint::Struct,
            "deserialize_option" => SerdeHint::Option,
            _ => return None,
        }),
        ("SeqAccess", "next_element") => SerdeOp::Next(SerdeNext::Element),
        ("MapAccess", "next_key") => SerdeOp::Next(SerdeNext::Key),
        ("MapAccess", "next_value") => SerdeOp::Next(SerdeNext::Value),
        ("MapAccess", "next_entry") => SerdeOp::Next(SerdeNext::Entry),
        ("Serializer", name) => match name {
            "serialize_bool" => SerdeOp::Write(SerdeScalar::Bool),
            "serialize_i8" | "serialize_i16" | "serialize_i32" | "serialize_i64" | "serialize_i128"
            | "serialize_u8" | "serialize_u16" | "serialize_u32" | "serialize_u64" | "serialize_u128" => {
                SerdeOp::Write(SerdeScalar::Int)
            }
            "serialize_f32" => SerdeOp::Write(SerdeScalar::Float { single: true }),
            "serialize_f64" => SerdeOp::Write(SerdeScalar::Float { single: false }),
            "serialize_char" => SerdeOp::Write(SerdeScalar::Char),
            "serialize_str" => SerdeOp::Write(SerdeScalar::Str),
            "serialize_unit" | "serialize_none" | "serialize_unit_struct" => SerdeOp::Write(SerdeScalar::Null),
            "serialize_some" => SerdeOp::WriteValue { at: 1 },
            "serialize_newtype_struct" => SerdeOp::WriteValue { at: 2 },
            "serialize_unit_variant" => SerdeOp::UnitVariant,
            "serialize_newtype_variant" => SerdeOp::NewtypeVariant,
            "collect_str" => SerdeOp::CollectStr,
            "collect_seq" => SerdeOp::CollectSeq,
            "collect_map" => SerdeOp::CollectMap,
            "serialize_tuple_variant" => SerdeOp::Begin(SerdeCompound::TupleVariant),
            "serialize_struct_variant" => SerdeOp::Begin(SerdeCompound::StructVariant),
            "serialize_seq" => SerdeOp::Begin(SerdeCompound::Seq),
            "serialize_tuple" => SerdeOp::Begin(SerdeCompound::Tuple),
            "serialize_tuple_struct" => SerdeOp::Begin(SerdeCompound::TupleStruct),
            "serialize_map" => SerdeOp::Begin(SerdeCompound::Map),
            "serialize_struct" => SerdeOp::Begin(SerdeCompound::Struct),
            _ => return None,
        },
        (name, "end") => SerdeOp::End(compound(name)?),
        (name, "serialize_element") if compound(name).is_some() => SerdeOp::Element,
        // A struct's field, its key and its value; a tuple's, its value.
        (name, "serialize_field") => match compound(name)? {
            SerdeCompound::Struct | SerdeCompound::StructVariant => SerdeOp::Field,
            _ => SerdeOp::Element,
        },
        (name, "skip_field") if compound(name).is_some() => SerdeOp::SkipField,
        ("SerializeMap", "serialize_entry") => SerdeOp::Entry,
        ("SerializeMap", "serialize_key") => SerdeOp::Key,
        ("SerializeMap", "serialize_value") => SerdeOp::Value,
        ("Error", "custom") if matches!(tcx.item_name(tcx.parent(trait_id)).as_str(), "ser" | "de") => SerdeOp::Custom,
        ("Error", name) if tcx.item_name(tcx.parent(trait_id)).as_str() == "de" => SerdeOp::DeError(match name {
            "invalid_type" => SerdeError::InvalidType,
            "invalid_value" => SerdeError::InvalidValue,
            "invalid_length" => SerdeError::InvalidLength,
            "unknown_variant" => SerdeError::UnknownVariant,
            "unknown_field" => SerdeError::UnknownField,
            "missing_field" => SerdeError::MissingField,
            "duplicate_field" => SerdeError::DuplicateField,
            _ => return None,
        }),
        _ => return None,
    })
}

/// Recognized serde_json calls carry type facts, never lowered operands.
pub(super) enum JsonCall<'tcx> {
    ToValue(Ty<'tcx>),
    FromValue(Ty<'tcx>),
    Index,
    Equal {
        other: Ty<'tcx>,
        value_first: bool,
        negate: bool,
    },
    Convert {
        to: Ty<'tcx>,
        from: Ty<'tcx>,
        json: Json,
    },
    Default,
    Method(JsonMethod),
    /// serde's data model, of a hand-written impl (ADR 0459), its call's
    /// arguments' types.
    Serde(SerdeOp, ty::GenericArgsRef<'tcx>),
}

impl<'a, 'tcx> Recognition<'a, 'tcx> {
    pub(super) fn json_call(&self, def_id: DefId, generic_args: ty::GenericArgsRef<'tcx>) -> Option<JsonCall<'tcx>> {
        let tcx = self.tcx;
        if let Some(op) = serde_op(tcx, def_id) {
            return Some(JsonCall::Serde(op, generic_args));
        }
        let name = tcx.item_name(def_id);
        if tcx.crate_name(def_id.krate).as_str() == "serde_json" && tcx.trait_of_assoc(def_id).is_none() {
            match name.as_str() {
                "to_value" => {
                    return Some(JsonCall::ToValue(
                        generic_args.types().next().expect("`to_value::<T>`").peel_refs(),
                    ));
                }
                // `serde_json::from_value::<T>(v)`: `T` read from the `Value`.
                "from_value" => {
                    return Some(JsonCall::FromValue(
                        generic_args.types().next().expect("`from_value::<T>`"),
                    ));
                }
                _ => {}
            }
        }
        if let Some(trait_id) = tcx.trait_of_assoc(def_id) {
            let tr = ty::TraitRef::from_assoc(tcx, trait_id, generic_args);
            let (this, other) = (tr.self_ty(), tr.args.types().nth(1));
            let this_value = self.json_type(this) == Some(Json::Value);
            let other_value = other.is_some_and(|o| self.json_type(o) == Some(Json::Value));
            if tcx.is_lang_item(trait_id, LangItem::Index) && this_value {
                return Some(JsonCall::Index);
            }
            if tcx.is_lang_item(trait_id, LangItem::PartialEq)
                && let Some(other) = other
                && this_value != other_value
            {
                return Some(JsonCall::Equal {
                    other: if this_value { other } else { this },
                    value_first: this_value,
                    negate: name.as_str() == "ne",
                });
            }
            let converting = match other {
                Some(other) if tcx.is_diagnostic_item(sym::From, trait_id) => Some((this, other)),
                Some(other) if tcx.is_diagnostic_item(sym::Into, trait_id) => Some((other, this)),
                _ => None,
            };
            if let Some((to, from)) = converting
                && let Some(json) = self.json_type(to)
            {
                return Some(JsonCall::Convert { to, from, json });
            }
            if tcx.is_diagnostic_item(Symbol::intern("Default"), trait_id) && this_value {
                return Some(JsonCall::Default);
            }
            return None;
        }
        let imp = tcx.inherent_impl_of_assoc(def_id)?;
        let owner = tcx.type_of(imp).instantiate_identity().skip_normalization();
        match self.json_type(owner)? {
            json @ (Json::Value | Json::Number) => Some(JsonCall::Method(JsonMethod::recognize(json, name))),
            Json::Map => None,
        }
    }
}

/// Inherent Value/Number operations, selected before operand emission.
pub(super) enum JsonMethod {
    NumberFromF64,
    NumberAsF64,
    /// `as_u64()` and `as_i64()`: `"u64"` or `"i64"`, a BigInt (ADR 0086).
    NumberAsInt(&'static str),
    NumberKind(&'static str),
    NumberIsI64,
    IsNull,
    IsTag(&'static str),
    IsNumber(&'static str),
    AsTag(&'static str),
    AsF64,
    AsInt(&'static str),
    Get,
    Unsupported {
        owner: &'static str,
        name: Symbol,
    },
}

impl JsonMethod {
    fn recognize(json: Json, name: Symbol) -> Self {
        if json == Json::Number {
            return match name.as_str() {
                "from_f64" => Self::NumberFromF64,
                "as_f64" => Self::NumberAsF64,
                "as_u64" => Self::NumberAsInt("u64"),
                "as_i64" => Self::NumberAsInt("i64"),
                "is_f64" => Self::NumberKind("f"),
                "is_u64" => Self::NumberKind("u"),
                "is_i64" => Self::NumberIsI64,
                _ => Self::Unsupported { owner: "Number", name },
            };
        }
        match name.as_str() {
            "is_null" => Self::IsNull,
            "is_boolean" => Self::IsTag("Bool"),
            "is_number" => Self::IsTag("Number"),
            "is_string" => Self::IsTag("String"),
            "is_array" => Self::IsTag("Array"),
            "is_object" => Self::IsTag("Object"),
            "is_f64" => Self::IsNumber("f64"),
            "is_u64" => Self::IsNumber("u64"),
            "is_i64" => Self::IsNumber("i64"),
            "as_bool" => Self::AsTag("Bool"),
            "as_str" => Self::AsTag("String"),
            "as_array" | "as_array_mut" => Self::AsTag("Array"),
            "as_object" | "as_object_mut" => Self::AsTag("Object"),
            "as_number" => Self::AsTag("Number"),
            "as_f64" => Self::AsF64,
            "as_u64" => Self::AsInt("u64"),
            "as_i64" => Self::AsInt("i64"),
            "get" | "get_mut" => Self::Get,
            _ => Self::Unsupported { owner: "Value", name },
        }
    }
}

/// The established Value representation for one source type. Container elements
/// are recognized recursively when lowering their conversion functions.
pub(super) enum JsonConversion<'tcx> {
    Tag(&'static str),
    Null,
    Float,
    Integer,
    Same,
    Vector(Ty<'tcx>),
    Option(Ty<'tcx>),
}

impl<'a, 'tcx> Recognition<'a, 'tcx> {
    /// `T`, for an `Option<T>`.
    pub(super) fn option_of(&self, ty: Ty<'tcx>) -> Option<Ty<'tcx>> {
        match ty.kind() {
            ty::Adt(adt, args) if self.tcx.is_lang_item(adt.did(), LangItem::Option) => args.types().next(),
            _ => None,
        }
    }

    pub(super) fn json_conversion(&self, from: Ty<'tcx>) -> Option<JsonConversion<'tcx>> {
        let from = from.peel_refs();
        if self.is_string_like(from) && !from.is_char() {
            return Some(JsonConversion::Tag("String"));
        }
        if from.is_bool() {
            return Some(JsonConversion::Tag("Bool"));
        }
        if from.is_unit() {
            return Some(JsonConversion::Null);
        }
        match Num::of(from) {
            // An `f32`'s `Value` is its `f64`, as serde_json's `Number::from_f32` is
            // (ADR 0122).
            Some(Num::F64 | Num::F32) => return Some(JsonConversion::Float),
            Some(_) => return Some(JsonConversion::Integer),
            None => {}
        }
        match self.json_type(from) {
            Some(Json::Map) => return Some(JsonConversion::Tag("Object")),
            Some(Json::Number) => return Some(JsonConversion::Tag("Number")),
            Some(Json::Value) => return Some(JsonConversion::Same),
            None => {}
        }
        if let ty::Adt(_, args) = from.kind()
            && self.is_std_type(from, StdItem::Vec)
        {
            return Some(JsonConversion::Vector(args.type_at(0)));
        }
        self.option_of(from).map(JsonConversion::Option)
    }

    /// The scalar comparison category used by serde_json's PartialEq impls.
    pub(super) fn json_comparison(&self, other: Ty<'tcx>) -> Option<&'static str> {
        let other = other.peel_refs();
        if self.is_string_like(other) {
            Some("String")
        } else if other.is_bool() {
            Some("Bool")
        } else {
            match Num::of(other)? {
                Num::F64 => Some("f64"),
                // serde_json's `as_f32()`, the number `as f32` (ADR 0122).
                Num::F32 => Some("f32"),
                n if n.signed() => Some("i64"),
                _ => Some("u64"),
            }
        }
    }
}

pub(super) enum WriteCall {
    Text,
    /// `f.pad(s)`: `s`, given the `Formatter`'s options (ADR 0143).
    Pad,
    /// `f.pad_integral(non_negative, prefix, digits)`: a number's digits, as
    /// a number's are given the `Formatter`'s options.
    PadIntegral,
    Display,
    Debug,
    /// `LowerHex::fmt(x, f)` and the like: of a type of the crate's, its impl's
    /// (ADR 0165).
    OtherFmt,
    StructFields,
    TupleFields,
    Struct,
    Tuple,
    Function,
    Trait,
    Unsupported,
}

#[derive(Clone, Copy, PartialEq)]
pub(super) enum SkipPredicate {
    None,
    Some,
    Empty,
}

impl<'a, 'tcx> Recognition<'a, 'tcx> {
    /// `&mut Formatter<'_>`.
    /// `fmt::Write`'s `write_str(f, s)` of a `Formatter`, called through the
    /// trait: its `&mut Self` is the `Formatter`, as `f.write_str(s)`'s is.
    pub(super) fn fmt_write_on_formatter(&self, def_id: DefId, args: ty::GenericArgsRef<'tcx>) -> bool {
        self.tcx
            .trait_of_assoc(def_id)
            .is_some_and(|t| self.tcx.is_diagnostic_item(Symbol::intern("FmtWrite"), t))
            && args
                .types()
                .next()
                .is_some_and(|this| self.is_std_type(this, StdItem::Formatter))
    }

    /// `f.write_str(s)`, `write!(f, ..)` or `f.write_char(c)` of a
    /// `Formatter`, inherent or `fmt::Write`'s.
    pub(super) fn formatter_write(&self, def_id: DefId, args: ty::GenericArgsRef<'tcx>) -> bool {
        let inherent = (self.tcx.inherent_impl_of_assoc(def_id)).is_some_and(|imp| {
            self.is_std_type(
                self.tcx.type_of(imp).instantiate_identity().skip_normalization(),
                StdItem::Formatter,
            )
        });
        (inherent && matches!(self.tcx.item_name(def_id).as_str(), "write_str" | "write_fmt"))
            || self.fmt_write_on_formatter(def_id, args)
    }

    /// A `Formatter`, whose text is a string in JS (ADR 0054).
    pub(super) fn is_formatter_type(&self, ty: Ty<'tcx>) -> bool {
        self.is_std_type(ty, StdItem::Formatter)
    }

    fn is_formatter(&self, ty: Ty<'tcx>) -> bool {
        matches!(ty.kind(), ty::Ref(_, inner, Mutability::Mut)
            if self.is_std_type(*inner, StdItem::Formatter))
    }
    pub(super) fn display_trait(&self) -> DefId {
        self.tcx
            .get_diagnostic_item(Symbol::intern("Display"))
            .expect("std has `Display`")
    }
    pub(super) fn is_fmt_result(&self, ty: Ty<'tcx>) -> bool {
        is_fmt_result_type(self.tcx, ty)
    }
    /// Is `ty` a `fmt::Error`, a `fmt::Result`'s `Err` (ADR 0187)?
    pub(super) fn is_fmt_error(&self, ty: Ty<'tcx>) -> bool {
        matches!(fmt_result(self.tcx).map(|result| *result.kind()), Some(ty::Adt(_, args))
            if self.tcx.erase_and_anonymize_regions(ty) == self.tcx.erase_and_anonymize_regions(args.type_at(1)))
    }
    /// Whether `def_id`, a function or a closure, writes (ADR 0054): which
    /// of its parameters is its `Formatter`, given it returns a `fmt::Result`.
    pub(super) fn formatter_param(&self, def_id: DefId) -> Option<usize> {
        if self.tcx.def_kind(def_id) == DefKind::Closure {
            let ty = self.tcx.type_of(def_id).instantiate_identity().skip_normalization();
            let ty::Closure(_, args) = ty.kind() else { return None };
            let sig = args.as_closure().sig().skip_binder();
            return self.writes_to(sig.inputs()[0].tuple_fields().as_slice(), sig.output());
        }
        let sig = self.tcx.fn_sig(def_id).skip_binder().skip_binder();
        self.writes_to(sig.inputs(), sig.output())
    }

    /// Of a call of a callable, `Fn::call(&g, (s, f))` given its result's
    /// type: which of its arguments is the `Formatter` it writes to, where
    /// it's a writer, as a writer function's is (ADR 0054).
    pub(super) fn callable_formatter(
        &self,
        def_id: DefId,
        args: ty::GenericArgsRef<'tcx>,
        output: Ty<'tcx>,
    ) -> Option<usize> {
        let fn_trait = self.tcx.trait_of_assoc(def_id)?;
        self.tcx.fn_trait_kind_from_def_id(fn_trait)?;
        let ty::Tuple(given) = args.type_at(1).kind() else {
            return None;
        };
        self.writes_to(given.as_slice(), output)
    }

    /// Which of `inputs` is the `Formatter` a writer writes to, of one
    /// that returns `output`, a `fmt::Result`.
    fn writes_to(&self, inputs: &[Ty<'tcx>], output: Ty<'tcx>) -> Option<usize> {
        if !self.is_fmt_result(output) {
            return None;
        }
        inputs.iter().position(|&t| self.is_formatter(t))
    }

    pub(super) fn write_call(&self, def_id: DefId, known_function: bool) -> WriteCall {
        let tcx = self.tcx;
        let trait_id = tcx.trait_of_assoc(def_id);
        let is_trait = |name: &str| trait_id.is_some_and(|t| tcx.is_diagnostic_item(Symbol::intern(name), t));
        let owner = tcx
            .inherent_impl_of_assoc(def_id)
            .map(|imp| tcx.type_of(imp).instantiate_identity().skip_normalization());
        let on_formatter = owner.is_some_and(|t| self.is_std_type(t, StdItem::Formatter));
        match tcx.item_name(def_id).as_str() {
            "write_fmt" | "write_str" | "write_char" if on_formatter || is_trait("FmtWrite") => WriteCall::Text,
            "pad" if on_formatter => WriteCall::Pad,
            "pad_integral" if on_formatter => WriteCall::PadIntegral,
            "fmt" if is_trait("Display") => WriteCall::Display,
            "fmt" if is_trait("Debug") => WriteCall::Debug,
            "fmt" if trait_id.is_some_and(|t| is_other_fmt_trait(tcx, t)) => WriteCall::OtherFmt,
            "debug_struct_fields_finish" if on_formatter => WriteCall::StructFields,
            "debug_tuple_fields_finish" if on_formatter => WriteCall::TupleFields,
            name if on_formatter && name.starts_with("debug_struct_field") && name.ends_with("_finish") => {
                WriteCall::Struct
            }
            name if on_formatter && name.starts_with("debug_tuple_field") && name.ends_with("_finish") => {
                WriteCall::Tuple
            }
            _ if known_function && trait_id.is_none() => WriteCall::Function,
            _ if trait_id.is_some_and(|t| t.is_local()) => WriteCall::Trait,
            _ => WriteCall::Unsupported,
        }
    }

    pub(super) fn skip_predicate(&self, function: DefId, ty: Ty<'tcx>) -> Option<SkipPredicate> {
        let standard = matches!(self.tcx.crate_name(function.krate).as_str(), "core" | "alloc");
        match self.tcx.item_name(function).as_str() {
            "is_none" if standard && self.option_of(ty).is_some() => Some(SkipPredicate::None),
            "is_some" if standard && self.option_of(ty).is_some() => Some(SkipPredicate::Some),
            "is_empty" if standard && (self.is_vec_like(ty.peel_refs()) || self.is_string_like(ty)) => {
                Some(SkipPredicate::Empty)
            }
            _ => None,
        }
    }

    pub(super) fn skips_none(&self, function: DefId) -> bool {
        self.tcx.crate_name(function.krate).as_str() == "core" && self.tcx.item_name(function).as_str() == "is_none"
    }
}

impl<'a, 'tcx> Recognition<'a, 'tcx> {
    pub(super) fn is_parse_error(&self, ty: Ty<'tcx>) -> bool {
        matches!(ty.kind(), ty::Adt(adt, _) if self.tcx.crate_name(adt.did().krate) == rustc_span::sym::core
            && ["ParseIntError", "ParseFloatError", "ParseBoolError", "ParseCharError", "TryFromIntError", "BorrowError", "BorrowMutError"]
                .contains(&self.tcx.item_name(adt.did()).as_str()))
    }

    /// A lock's `TryLockError`: `WouldBlock`, or `Poisoned` of a `PoisonError`.
    pub(super) fn is_try_lock_error(&self, ty: Ty<'tcx>) -> bool {
        matches!(ty.kind(), ty::Adt(adt, _) if std_path(self.tcx, adt.did()) == "std::sync::TryLockError")
    }

    /// A `Utf8Error`, `false`, or a `FromUtf8Error`, `true`: the objects the
    /// runtime makes of them (ADR 0172).
    pub(super) fn utf8_error(&self, ty: Ty<'tcx>) -> Option<bool> {
        let ty::Adt(adt, _) = ty.kind() else {
            return None;
        };
        match std_path(self.tcx, adt.did()).as_str() {
            "std::str::Utf8Error" => Some(false),
            "std::string::FromUtf8Error" => Some(true),
            _ => None,
        }
    }

    /// A `Path` or a `PathBuf`, whose `==` and ordering are its components'
    /// (ADR 0401), where an `OsStr`'s are its text's.
    pub(super) fn is_path(&self, ty: Ty<'tcx>) -> bool {
        matches!(ty.peel_refs().kind(), ty::Adt(adt, _)
            if matches!(std_path(self.tcx, adt.did()).as_str(), "std::path::Path" | "std::path::PathBuf"))
    }

    /// A `Path`, a `PathBuf`, what `display()` shows of one, or an `OsStr` or
    /// `OsString`: its text, a JS string (ADR 0173). Not a string otherwise: a
    /// path's `==` compares its components, `a/b` and `a//b` equal.
    pub(super) fn is_path_like(&self, ty: Ty<'tcx>) -> bool {
        matches!(ty.peel_refs().kind(), ty::Adt(adt, _) if matches!(
            std_path(self.tcx, adt.did()).as_str(),
            "std::path::Path" | "std::path::PathBuf" | "std::path::Display" | "std::ffi::OsStr" | "std::ffi::OsString"
        ))
    }

    /// `std::num::Wrapping<T>`'s `T`: a number in a `[x]` (ADR 0175).
    pub(super) fn wrapping_of(&self, ty: Ty<'tcx>) -> Option<Ty<'tcx>> {
        match ty.kind() {
            ty::Adt(_, args) if self.is_std_type(ty, StdItem::Wrapping) => Some(args.type_at(0)),
            _ => None,
        }
    }

    /// A `OnceLock` or `LazyLock`, std's, not a `OnceCell` or `LazyCell`,
    /// `core`'s: their reentrant inits differ (ADRs 0317, 0318).
    pub(super) fn is_lock_cell(&self, ty: Ty<'tcx>) -> bool {
        matches!(ty.kind(), ty::Adt(adt, _) if self.tcx.crate_name(adt.did().krate) == sym::std)
            && (self.is_std_type(ty, StdItem::OnceCell) || self.is_std_type(ty, StdItem::LazyCell))
    }

    /// `Rc::new` or an `Rc`'s `Deref`, each `Std::Same` of an `Rc` that's
    /// its value (ADR 0023): whether it's `new`, and what it points at, for
    /// a counted one's (ADR 0320).
    pub(super) fn rc_same(&self, def_id: DefId, args: ty::GenericArgsRef<'tcx>) -> Option<(bool, Ty<'tcx>)> {
        let tcx = self.tcx;
        if tcx.is_diagnostic_item(Symbol::intern("deref_method"), def_id) {
            return rc_pointee(tcx, args.type_at(0)).map(|pointee| (false, pointee));
        }
        // `Rc::from(x)` and `x.into()`, of what `rc_made_of` says.
        if let Some(trait_) = tcx.trait_of_assoc(def_id)
            && let Some((from, to)) = conversion_sides(tcx, trait_, args)
            && tcx.is_diagnostic_item(sym::From, trait_) | tcx.is_diagnostic_item(sym::Into, trait_)
        {
            return rc_made_of(tcx, from, to).map(|pointee| (true, pointee));
        }
        let imp = tcx.inherent_impl_of_assoc(def_id)?;
        let owner = tcx.type_of(imp).instantiate_identity().skip_normalization();
        (rc_pointee(tcx, owner).is_some() && tcx.item_name(def_id).as_str() == "new").then(|| (true, args.type_at(0)))
    }

    /// A slice's `chunks_exact(n)` or `rchunks_exact(n)`, whose `remainder()`
    /// is what's left (ADR 0324).
    pub(super) fn is_exact_chunks(&self, ty: Ty<'tcx>) -> bool {
        matches!(ty.kind(), ty::Adt(adt, _) if matches!(
            std_path(self.tcx, adt.did()).as_str(),
            "std::slice::ChunksExact"
                | "std::slice::RChunksExact"
                | "std::slice::ChunksExactMut"
                | "std::slice::RChunksExactMut"
        ))
    }

    /// A `Cow`, `{ TAG, _0 }` (ADR 0033): what it borrowed, or owns.
    pub(super) fn is_cow(&self, ty: Ty<'tcx>) -> bool {
        matches!(ty.kind(), ty::Adt(adt, _) if std_path(self.tcx, adt.did()) == "std::borrow::Cow")
    }

    /// What a `Cow` borrows and what it owns, where they're one JS value,
    /// as std's `ToOwned` makes them (ADR 0319): `T` and `T`, `str` and
    /// `String`, `[T]` and `Vec<T>`, `Path` and `PathBuf`. `None` for another.
    pub(super) fn cow_parts(&self, ty: Ty<'tcx>) -> Option<(Ty<'tcx>, Ty<'tcx>)> {
        let ty::Adt(adt, args) = ty.kind() else { return None };
        if !self.is_cow(ty) {
            return None;
        }
        let borrowed = args.types().next()?;
        let owned_variant = adt.variants().iter().nth(1)?;
        let owned = self.field_ty(owned_variant.fields.iter().next()?, args);
        let same = owned == borrowed
            || borrowed.is_str() && self.is_lang_adt(owned, LangItem::String)
            // `Path` and `PathBuf`, `OsStr` and `OsString`: text (ADR 0173).
            || self.is_path_like(borrowed) && self.is_path_like(owned)
            || matches!(borrowed.kind(), ty::Slice(item) if self.is_std_type(owned, StdItem::Vec)
                && matches!(owned.kind(), ty::Adt(_, vec) if vec.type_at(0) == *item));
        same.then_some((borrowed, owned))
    }

    pub(super) fn is_json_error(&self, ty: Ty<'tcx>) -> bool {
        matches!(ty.kind(), ty::Adt(adt, _) if self.tcx.crate_name(adt.did().krate).as_str() == "serde_json"
            && self.tcx.item_name(adt.did()).as_str() == "Error")
    }

    pub(super) fn is_reverse(&self, ty: Ty<'tcx>) -> bool {
        matches!(ty.kind(), ty::Adt(adt, _) if self.tcx.crate_name(adt.did().krate) == rustc_span::sym::core
            && self.tcx.item_name(adt.did()).as_str() == "Reverse")
    }

    /// Which `StepOp` an `Iterator` method of `ty` is, one that steps a
    /// `$iter` (ADR 0071): `peekable()`, and `cycle()` of one over an array,
    /// whose clone is what it has left (ADR 0181).
    fn iterator_step(&self, name: &str, ty: Ty<'tcx>) -> Option<StepOp> {
        match name {
            "peekable" => Some(StepOp::Peekable),
            "cycle" if self.array_source(ty.peel_refs()).is_some() => Some(StepOp::Cycle),
            _ => None,
        }
    }

    /// std's iterator over an array's items, whose JS value is the array
    /// (ADR 0061): `Some(false)` of `slice::Iter`, which borrows them, and
    /// `Some(true)` of `array::IntoIter` and `vec::IntoIter`, which own them,
    /// of a vector's or a heap's `drain(..)`, the array of what it took out,
    /// which owns them too (ADR 0098), of a string's `bytes()`, an array of its bytes
    /// (ADR 0034), and of `iter::Empty`, `[]` (ADR 0128).
    pub(super) fn array_source(&self, ty: Ty<'tcx>) -> Option<bool> {
        let ty = self.reveal(ty);
        let ty::Adt(adt, _) = ty.kind() else { return None };
        if self.is_std_type(ty, StdItem::SliceIter) {
            return Some(false);
        }
        if self.is_str_bytes(ty) {
            return Some(true);
        }
        let owning = [sym::alloc, sym::std].contains(&self.tcx.crate_name(adt.did().krate))
            && matches!(
                std_path(self.tcx, adt.did()).as_str(),
                "std::vec::IntoIter"
                    | "std::vec::Drain"
                    | "std::collections::vec_deque::Drain"
                    | "std::collections::binary_heap::Drain"
                    | "std::collections::hash_map::Drain"
                    | "std::collections::hash_set::Drain"
            );
        (self.is_std_type(ty, StdItem::ArrayIntoIter) || owning || self.is_iter_empty(ty)).then_some(true)
    }

    /// std's iterator adapter, `iter::Map` and the like: a JS iterator's,
    /// whose fields JS doesn't have.
    pub(super) fn is_iter_adapter(&self, ty: Ty<'tcx>) -> bool {
        matches!(ty.kind(), ty::Adt(adt, _) if std_path(self.tcx, adt.did()).starts_with("std::iter::"))
    }

    /// The name std's `Debug` gives an iterator over an array, which shows
    /// the slice it has left, `IntoIter([2, 3])`: a slice's `Iter`, and a
    /// `Vec`'s and an array's `IntoIter` and `Drain` (ADR 0473).
    pub(crate) fn shows_its_slice(&self, ty: Ty<'tcx>) -> Option<&'static str> {
        let ty::Adt(adt, _) = ty.kind() else { return None };
        if self.is_std_type(ty, StdItem::SliceIter) {
            return Some("Iter");
        }
        if self.is_std_type(ty, StdItem::ArrayIntoIter) {
            return Some("IntoIter");
        }
        match std_path(self.tcx, adt.did()).as_str() {
            "std::vec::IntoIter" => Some("IntoIter"),
            "std::vec::Drain" => Some("Drain"),
            _ => None,
        }
    }

    /// What a `Fuse` steps, its `I`, of a `Fuse<I>` (ADR 0473).
    pub(crate) fn fuse_of(&self, ty: Ty<'tcx>) -> Option<Ty<'tcx>> {
        let ty::Adt(adt, args) = ty.kind() else { return None };
        (std_path(self.tcx, adt.did()) == "std::iter::Fuse").then(|| args.type_at(0))
    }

    /// Whether `ty`, a type rustc knows whole, is a `FusedIterator`, whose
    /// `Fuse` keeps it as it ends, as std's does (ADR 0473).
    pub(crate) fn is_fused_iterator(&self, ty: Ty<'tcx>) -> bool {
        let fused = self
            .tcx
            .require_lang_item(LangItem::FusedIterator, rustc_span::DUMMY_SP);
        let tr = self
            .tcx
            .erase_and_anonymize_regions(ty::TraitRef::new(self.tcx, fused, [ty]));
        self.tcx
            .codegen_select_candidate(self.typing_env.as_query_input(tr))
            .is_ok()
    }

    /// An iterator of std's whose drop is closing it, JS's `return()`, and
    /// what it holds that its closing drops (ADRs 0422, 0465): `iter::repeat`'s
    /// value; and a lazy adapter's, `take`'s, `map`'s, `zip`'s and the rest,
    /// of what its sources hold, each closable, an iterator that owns its
    /// items given it as `$closing`, whose closing drops what it hasn't given,
    /// or one that holds nothing, a slice's or a range's. The one list of
    /// what's closed to drop.
    pub(super) fn closed_on_drop(&self, ty: Ty<'tcx>) -> Option<Ty<'tcx>> {
        let ty = self.reveal(ty);
        let ty::Adt(adt, args) = ty.kind() else { return None };
        let mut sources = args.types();
        match std_path(self.tcx, adt.did()).as_str() {
            "std::iter::Repeat" => sources.next(),
            "std::iter::Take"
            | "std::iter::Skip"
            | "std::iter::Map"
            | "std::iter::Filter"
            | "std::iter::Enumerate"
            | "std::iter::SkipWhile"
            | "std::iter::TakeWhile"
            | "std::iter::StepBy"
            | "std::iter::Inspect"
            | "std::iter::Fuse" => self.closing_source(sources.next()?)?,
            "std::iter::Zip" | "std::iter::Chain" => {
                let held: Vec<Option<Ty<'tcx>>> = sources
                    .take(2)
                    .map(|source| self.closing_source(source))
                    .collect::<Option<_>>()?;
                match held.as_slice() {
                    [Some(a), Some(b)] => Some(Ty::new_tup(self.tcx, &[*a, *b])),
                    [Some(one), None] | [None, Some(one)] => Some(*one),
                    _ => None,
                }
            }
            _ => None,
        }
    }

    /// What closing an adapter's `source` drops: `Some` of what it holds, an
    /// owning iterator's items or what one closed to drop holds, a generic
    /// one itself, closed by its caller's drop, or the crate's, by its own,
    /// `Some(None)`
    /// of one that holds nothing, a slice's or a range's, `None` of another.
    fn closing_source(&self, source: Ty<'tcx>) -> Option<Option<Ty<'tcx>>> {
        if let Some(held) = self.owned_items(source).or_else(|| self.closed_on_drop(source)) {
            return Some(Some(held));
        }
        // A generic one, closed by the drop its caller gives, and the crate's
        // own, by its own drop.
        let generic = matches!(
            source.kind(),
            ty::Param(_)
                | ty::Alias(
                    _,
                    ty::AliasTy {
                        kind: ty::Projection { .. },
                        ..
                    }
                )
        );
        if generic || matches!(source.kind(), ty::Adt(adt, _) if !self.is_std(adt.did())) {
            return Some(Some(source));
        }
        (self.array_source(source) == Some(false) || self.range_kind(source).is_some()).then_some(None)
    }

    /// What an iterator that owns its items gives, `vec::IntoIter`'s,
    /// `drain(..)`'s, a map's `(K, V)`, or `option::IntoIter`'s, each of which
    /// it drops with it.
    pub(super) fn owned_items(&self, ty: Ty<'tcx>) -> Option<Ty<'tcx>> {
        let ty = self.reveal(ty);
        let ty::Adt(adt, args) = ty.kind() else { return None };
        if !(self.array_source(ty) == Some(true) || self.owns_one_item(ty)) {
            return None;
        }
        // A map's `drain()`'s, its entries, `(K, V)`.
        if std_path(self.tcx, adt.did()) == "std::collections::hash_map::Drain" {
            let mut types = args.types();
            return Some(Ty::new_tup(self.tcx, &[types.next()?, types.next()?]));
        }
        args.types().next()
    }

    /// What a value given as an iterable owns and drops with what it hasn't
    /// given, `zip(v)`'s `Vec`: an owning iterator's items, or a `Vec`'s, a
    /// `VecDeque`'s or an array's, given by value.
    pub(super) fn owned_into_items(&self, ty: Ty<'tcx>) -> Option<Ty<'tcx>> {
        if let Some(item) = self.owned_items(ty) {
            return Some(item);
        }
        match ty.kind() {
            ty::Array(item, _) => Some(*item),
            ty::Adt(_, args) if self.is_std_type(ty, StdItem::Vec) || self.is_std_type(ty, StdItem::VecDeque) => {
                args.types().next()
            }
            _ => None,
        }
    }

    /// An iterator that owns the one item it may give, `Some(x).into_iter()`
    /// or `iter::once(x)`: the array of none or one (ADR 0128).
    pub(super) fn owns_one_item(&self, ty: Ty<'tcx>) -> bool {
        let ty = self.reveal(ty);
        matches!(ty.kind(), ty::Adt(adt, _)
            if matches!(std_path(self.tcx, adt.did()).as_str(), "std::option::IntoIter" | "std::iter::Once"))
    }

    /// A B-tree's `iter()`, `keys()`, `range()` and the like: its items in
    /// order, an array's (ADR 0059), `$treeRange`'s of a range (ADR 0325). Not its `extract_if()`, which takes them as it's
    /// stepped, `$mapExtractIf`'s.
    pub(super) fn is_tree_iter(&self, ty: Ty<'tcx>) -> bool {
        let ty = self.reveal(ty);
        matches!(ty.kind(), ty::Adt(adt, _) if {
            let path = std_path(self.tcx, adt.did());
            (path.starts_with("std::collections::btree_map::") || path.starts_with("std::collections::btree_set::"))
                && !path.ends_with("::ExtractIf")
        })
    }

    /// A slice's `iter_mut()`'s, over its array's items, or handles on
    /// them (ADR 0099).
    pub(super) fn is_slice_iter_mut(&self, ty: Ty<'tcx>) -> bool {
        let ty = self.reveal(ty);
        matches!(ty.kind(), ty::Adt(adt, _) if std_path(self.tcx, adt.did()) == "std::slice::IterMut")
    }

    /// `cmp::max`'s name, `"max"`, or `cmp::min`'s, `"min"`: of what isn't a
    /// number, `Ord`'s method of that name's call (ADR 0136).
    /// A `Result`.
    pub(super) fn is_result(&self, ty: Ty<'tcx>) -> bool {
        self.is_std_type(ty, StdItem::Result)
    }

    /// `FromIterator`, the trait `collect` makes what it collects with.
    pub(super) fn is_from_iterator(&self, trait_id: DefId) -> bool {
        self.tcx.is_diagnostic_item(sym::FromIterator, trait_id)
    }

    /// `FromIterator::from_iter` and its arguments, `collection` made of
    /// `item`s given an array of them, `[A; 0]`, an `IntoIterator<Item =
    /// A>`: what `collect` into a collection of the crate's own calls, of
    /// values made already, a fallible `collect`'s.
    pub(super) fn values_from_iter(
        &self,
        collection: Ty<'tcx>,
        item: Ty<'tcx>,
    ) -> Option<(DefId, ty::GenericArgsRef<'tcx>)> {
        let trait_id = self.tcx.get_diagnostic_item(sym::FromIterator)?;
        let from_iter = (self.tcx.associated_items(trait_id).in_definition_order())
            .find(|i| i.is_fn() && i.name().as_str() == "from_iter")?
            .def_id;
        let values = Ty::new_array(self.tcx, item, 0);
        Some((
            from_iter,
            self.tcx.mk_args(&[collection.into(), item.into(), values.into()]),
        ))
    }

    /// The method of std's iterator traits `def_id` is: `Iterator`'s,
    /// `DoubleEndedIterator`'s `next_back`, `ExactSizeIterator`'s `len`,
    /// `IntoIterator`'s `into_iter` or `FromIterator`'s `from_iter`.
    pub(super) fn iterator_method(&self, def_id: DefId) -> Option<IterMethod> {
        let trait_id = self.tcx.trait_of_assoc(def_id)?;
        let name = self.tcx.item_name(def_id);
        Some(match name.as_str() {
            "into_iter" if is_std_def(self.tcx, trait_id, StdItem::IntoIterator) => IterMethod::IntoIter,
            "from_iter" if self.is_from_iterator(trait_id) => IterMethod::FromIter,
            "next_back" if self.double_ended_iterator() == Some(trait_id) => IterMethod::NextBack,
            "nth_back" if self.double_ended_iterator() == Some(trait_id) => IterMethod::NthBack,
            "rfold" if self.double_ended_iterator() == Some(trait_id) => IterMethod::RFold,
            "rfind" if self.double_ended_iterator() == Some(trait_id) => IterMethod::RFind,
            "len" if self.exact_size_iterator() == Some(trait_id) => IterMethod::Len,
            _ if !is_std_def(self.tcx, trait_id, StdItem::Iterator) => return None,
            "next" => IterMethod::Next,
            "size_hint" => IterMethod::SizeHint,
            "rev" => IterMethod::Rev,
            "peekable" => IterMethod::Peekable,
            "by_ref" => IterMethod::ByRef,
            "fuse" => IterMethod::Fuse,
            "map" => IterMethod::Map,
            "filter" => IterMethod::Filter,
            "skip_while" => IterMethod::SkipWhile,
            "flat_map" => IterMethod::FlatMap,
            "take" => IterMethod::Take,
            "skip" => IterMethod::Skip,
            "for_each" => IterMethod::ForEach,
            "any" => IterMethod::Any,
            "all" => IterMethod::All,
            "find" => IterMethod::Find,
            "position" => IterMethod::Position,
            "rposition" => IterMethod::RPosition,
            "eq" => IterMethod::Eq,
            "ne" => IterMethod::Ne,
            "enumerate" => IterMethod::Enumerate,
            "fold" => IterMethod::Fold,
            "reduce" => IterMethod::Reduce,
            "try_fold" => IterMethod::TryFold,
            "count" => IterMethod::Count,
            "sum" => IterMethod::Sum,
            "product" => IterMethod::Product,
            "copied" => IterMethod::Copied,
            "cloned" => IterMethod::Cloned,
            "max" => IterMethod::Max,
            "min" => IterMethod::Min,
            "last" => IterMethod::Last,
            "nth" => IterMethod::Nth,
            "partition" => IterMethod::Partition,
            "collect" => IterMethod::Collect,
            _ => return None,
        })
    }

    /// `env::args()`'s, an array of the process's arguments, which knows
    /// its length (ADR 0391).
    pub(super) fn is_process_items(&self, ty: Ty<'tcx>) -> bool {
        matches!(ty.kind(), ty::Adt(adt, _) if std_path(self.tcx, adt.did()) == "std::env::Args")
    }

    /// `env::VarError`, what `env::var` gives of a variable it hasn't:
    /// `"NotPresent"`, as an enum's unit variant is (ADR 0391).
    pub(super) fn is_var_error(&self, ty: Ty<'tcx>) -> bool {
        matches!(ty.kind(), ty::Adt(adt, _) if std_path(self.tcx, adt.did()) == "std::env::VarError")
    }

    /// `path::StripPrefixError`, what `strip_prefix` gives of a path that
    /// doesn't start with what's asked: `undefined`, as it holds nothing
    /// (ADR 0405).
    pub(super) fn is_strip_prefix_error(&self, ty: Ty<'tcx>) -> bool {
        matches!(ty.kind(), ty::Adt(adt, _) if std_path(self.tcx, adt.did()) == "std::path::StripPrefixError")
    }

    /// `VecDeque::iter()`'s, over the array a `VecDeque` is.
    pub(super) fn is_deque_iter(&self, ty: Ty<'tcx>) -> bool {
        let ty = self.reveal(ty);
        matches!(ty.kind(), ty::Adt(adt, _) if std_path(self.tcx, adt.did()) == "std::collections::vec_deque::Iter")
    }

    pub(super) fn is_array_iter(&self, ty: Ty<'tcx>) -> bool {
        let ty = self.reveal(ty);
        let ty::Adt(adt, _) = ty.kind() else { return false };
        let path = std_path(self.tcx, adt.did());
        let krate = self.tcx.crate_name(adt.did().krate);
        (krate == sym::core || krate == sym::alloc)
            && (path.contains("::iter::")
                || [
                    "std::slice::Iter",
                    "std::vec::IntoIter",
                    "std::str::SplitN",
                    "std::str::RSplitN",
                    "std::str::RSplit",
                    "std::str::SplitTerminator",
                    "std::str::MatchIndices",
                    "std::str::Matches",
                    "std::str::Chars",
                    "std::str::CharIndices",
                    "std::str::SplitWhitespace",
                    "std::str::Lines",
                    "std::array::IntoIter",
                    "std::collections::vec_deque::Iter",
                    "std::collections::vec_deque::IntoIter",
                    "std::collections::binary_heap::Iter",
                    "std::collections::binary_heap::IntoIter",
                    "std::option::Iter",
                    "std::option::IntoIter",
                    "std::result::IterMut",
                    // A slice's chunks and splits, copies or views (ADR 0335).
                    "std::slice::ChunksExact",
                    "std::slice::ChunksExactMut",
                    "std::slice::RChunks",
                    "std::slice::RChunksMut",
                    "std::slice::RChunksExact",
                    "std::slice::RChunksExactMut",
                    "std::slice::Split",
                    "std::slice::SplitMut",
                    "std::slice::SplitN",
                    "std::slice::SplitNMut",
                    "std::slice::RSplit",
                    "std::slice::RSplitMut",
                    "std::slice::RSplitN",
                    "std::slice::RSplitNMut",
                    "std::slice::SplitInclusive",
                    "std::slice::SplitInclusiveMut",
                    "std::slice::ChunkBy",
                    "std::slice::ChunkByMut",
                ]
                .contains(&path.as_str())
                || self.is_str_split(ty)
                || is_case_mapping(self.tcx, ty))
            // A map's `iter()`, `keys()` and `values()` are arrays too (ADR 0059).
            || ((krate == sym::alloc || krate == sym::std)
                && ["btree_map::Iter", "btree_map::IterMut", "btree_map::Keys", "btree_map::Values", "btree_map::ValuesMut", "btree_map::IntoIter", "btree_set::Iter", "btree_set::IntoIter"]
                    .iter()
                    .any(|name| path == format!("std::collections::{name}")))
            || (krate == sym::std
                && ["hash_map::Iter", "hash_map::IterMut", "hash_map::Keys", "hash_map::Values", "hash_map::ValuesMut", "hash_map::IntoIter", "hash_set::Iter", "hash_set::IntoIter"]
                    .iter()
                    .any(|name| path == format!("std::collections::{name}")))
    }

    pub(super) fn is_str_split(&self, ty: Ty<'tcx>) -> bool {
        matches!(ty.kind(), ty::Adt(adt, _) if self.tcx.crate_name(adt.did().krate) == sym::core
            && self.tcx.item_name(adt.did()).as_str() == "Split"
            && std_path(self.tcx, adt.did()).contains("str::"))
    }

    /// A field's type, of an ADT given `args`: a projection in it, `K::Value`,
    /// is the type it stands for, `Option<u32>`, where that's known, as in
    /// rustc's own types of places. Only a type with one is normalized, which
    /// erases its lifetimes, so one without stays the type it was.
    pub(super) fn field_ty(&self, field: &ty::FieldDef, args: ty::GenericArgsRef<'tcx>) -> Ty<'tcx> {
        let ty = field.ty(self.tcx, args).skip_normalization();
        if !rustc_middle::ty::TypeVisitableExt::has_aliases(&ty) {
            return ty;
        }
        self.tcx
            .try_normalize_erasing_regions(self.typing_env, ty::Unnormalized::new_wip(ty))
            .unwrap_or(ty)
    }

    /// A type only a caller knows: a type parameter, or an associated type of
    /// one, `<S as Source>::Item`, that isn't a type here (ADR 0106). Its
    /// dictionaries are the ones a function is given. One that is a type here,
    /// `<Count as Source>::Item` in an impl's signature, is that type.
    pub(super) fn is_unknown(&self, ty: Ty<'tcx>) -> bool {
        self.is_unknown_in(ty, self.typing_env)
    }

    /// `is_unknown`, of a type of another function's, whose parameters are
    /// its own: asked in its `typing_env`, as the caller's would find its
    /// parameters at other indexes, arrayvec's `CAP`.
    pub(super) fn is_unknown_in(&self, ty: Ty<'tcx>, typing_env: ty::TypingEnv<'tcx>) -> bool {
        match ty.kind() {
            ty::Param(_) => true,
            ty::Alias(
                _,
                ty::AliasTy {
                    kind: ty::Projection { .. },
                    ..
                },
            ) => self
                .tcx
                .try_normalize_erasing_regions(typing_env, ty::Unnormalized::new_wip(ty))
                .map_or(true, |known| {
                    matches!(
                        known.kind(),
                        ty::Param(_)
                            | ty::Alias(
                                _,
                                ty::AliasTy {
                                    kind: ty::Projection { .. },
                                    ..
                                }
                            )
                    )
                }),
            _ => false,
        }
    }

    /// A trait rust-js compiled: the crate's own, or a library's (ADR 0100).
    /// Any other is std's, whose dictionaries rust-js makes as it knows them.
    pub(super) fn is_rust_trait(&self, id: DefId) -> bool {
        id.is_local() || self.foreign.in_library(id)
    }

    /// A trait whose `dyn` is a pair, its value and its dictionary (ADR
    /// 0049): one rust-js compiled, or std's `Display`, `ToString`, `Error`,
    /// `io::Write`, `fmt::Write` or `Any` (ADRs 0141, 0331, 0446).
    pub(super) fn is_pair_trait(&self, id: DefId) -> bool {
        self.is_rust_trait(id) || self.is_std_pair_trait(id)
    }

    /// Whether a dictionary of `id`'s holds its supertraits': a pair trait's,
    /// as a `dyn` calls them, and `DerefMut`'s, its `Deref` (ADR 0394).
    pub(super) fn holds_supertraits(&self, id: DefId) -> bool {
        self.is_pair_trait(id) || self.tcx.is_lang_item(id, LangItem::DerefMut)
    }

    /// Of those, std's.
    pub(super) fn is_std_pair_trait(&self, id: DefId) -> bool {
        [
            StdItem::Display,
            StdItem::ToString,
            StdItem::Error,
            StdItem::IoWrite,
            StdItem::FmtWrite,
        ]
        .into_iter()
        .any(|item| is_std_def(self.tcx, id, item))
            || any_trait(self.tcx) == Some(id)
            // A `dyn Hasher`, its dictionary a hasher's (ADR 0463).
            || is_hasher(self.tcx, id)
    }

    /// A `dyn` of `predicates` that's a pair: of a pair trait, or of no
    /// trait but auto traits, `dyn Send`, whose dictionary is its drop alone.
    /// Each keeps its value's drop, its dictionary's `$drop`, as Rust's
    /// vtable does (ADR 0098).
    pub(super) fn is_pair_dyn(&self, predicates: &ty::List<ty::PolyExistentialPredicate<'tcx>>) -> bool {
        predicates.principal_def_id().is_none_or(|id| self.is_pair_trait(id))
    }

    pub(super) fn reveal(&self, ty: Ty<'tcx>) -> Ty<'tcx> {
        if !rustc_middle::ty::TypeVisitableExt::has_opaque_types(&ty) {
            return ty;
        }
        self.tcx
            .try_normalize_erasing_regions(self.typing_env, ty::Unnormalized::new_wip(ty))
            .unwrap_or(ty)
    }

    /// The `Clone` impl of the crate's that a `ty`'s is, if it has a
    /// `clone_from` of its own, which `clone_from_slice` calls (ADR 0324).
    pub(super) fn own_clone_from(&self, ty: Ty<'tcx>) -> bool {
        let clone = self.tcx.require_lang_item(LangItem::Clone, rustc_span::DUMMY_SP);
        let tr = ty::TraitRef::new(self.tcx, clone, [self.tcx.erase_and_anonymize_regions(ty)]);
        matches!(self.tcx.codegen_select_candidate(self.typing_env.as_query_input(tr)),
            Ok(ImplSource::UserDefined(imp)) if !self.is_std(imp.impl_def_id) && self.has_clone_from(imp.impl_def_id))
    }

    /// Whether a `clone_from` of the crate's may run in a `clone_from` of a
    /// `ty` but its own: one of a type inside it, or of what a type parameter
    /// stands for, where any `Clone` impl of the crate's has one.
    pub(super) fn reaches_clone_from(&self, ty: Ty<'tcx>) -> bool {
        let clone = self.tcx.require_lang_item(LangItem::Clone, rustc_span::DUMMY_SP);
        let any = self
            .trait_impls
            .iter()
            .any(|&imp| self.tcx.impl_opt_trait_id(imp) == Some(clone) && self.has_clone_from(imp));
        any && ty.walk().skip(1).any(|part| {
            part.as_type().is_some_and(|t| {
                matches!(t.kind(), ty::Param(_) | ty::Alias(..) | ty::Dynamic(..)) || self.own_clone_from(t)
            })
        }) || any && matches!(ty.kind(), ty::Param(_) | ty::Alias(..) | ty::Dynamic(..))
    }

    /// Does the impl `imp` write its own `clone_from`?
    pub(super) fn has_clone_from(&self, imp: DefId) -> bool {
        self.tcx
            .associated_items(imp)
            .in_definition_order()
            .any(|item| item.name().as_str() == "clone_from")
    }

    /// `TypeId`, which rust-js holds as its type's key, a string (ADR 0331).
    pub(super) fn is_type_id(&self, ty: Ty<'tcx>) -> bool {
        self.is_lang_adt(ty, LangItem::TypeId)
    }

    /// `mem::Discriminant<T>`, what `mem::discriminant` gives: its variant's
    /// discriminant, a number (ADR 0393).
    pub(super) fn is_discriminant(&self, ty: Ty<'tcx>) -> bool {
        is_discriminant_type(self.tcx, ty)
    }

    /// A `dyn Any`, `+ Send` or not (ADR 0331).
    pub(super) fn is_dyn_any(&self, ty: Ty<'tcx>) -> bool {
        matches!(ty.kind(), ty::Dynamic(traits, ..)
            if traits.principal_def_id().is_some_and(|id| self.tcx.is_diagnostic_item(Symbol::intern("Any"), id)))
    }

    /// A `PeekMut<T>`'s `T`, a guard of a heap's top (ADR 0333).
    pub(super) fn peek_mut_of(&self, ty: Ty<'tcx>) -> Option<Ty<'tcx>> {
        match ty.kind() {
            ty::Adt(adt, args) if std_path(self.tcx, adt.did()) == "std::collections::binary_heap::PeekMut" => {
                args.types().next()
            }
            _ => None,
        }
    }

    /// A `MaybeUninit<T>`'s `T`, which it is in JS, or `undefined` (ADR 0332).
    pub(super) fn uninit_of(&self, ty: Ty<'tcx>) -> Option<Ty<'tcx>> {
        match ty.kind() {
            ty::Adt(adt, args) if is_std_def(self.tcx, adt.did(), StdItem::MaybeUninit) => args.types().next(),
            _ => None,
        }
    }

    /// A type that.s the value it holds in JS, whose field is it: a
    /// `MaybeUninit` (ADR 0332), and what std makes one of, `ManuallyDrop`
    /// and `MaybeDangling`.
    pub(super) fn transparent(&self, ty: Ty<'tcx>) -> bool {
        self.uninit_of(ty).is_some()
            || self.is_lang_adt(ty, LangItem::ManuallyDrop)
            || self.is_lang_adt(ty, LangItem::MaybeDangling)
            // A `NonZero`, its number (ADR 0177), in std's `NonZeroU8Inner`.
            || matches!(ty.kind(), ty::Adt(adt, _) if is_non_zero(adt.did())
                || std_path(self.tcx, adt.did()).starts_with("std::num::niche_types::"))
    }

    /// A `Pin<P>`'s `P`, the pointer it is in JS (ADR 0329).
    pub(super) fn pinned(&self, ty: Ty<'tcx>) -> Option<Ty<'tcx>> {
        match ty.kind() {
            ty::Adt(adt, args) if self.tcx.is_lang_item(adt.did(), LangItem::Pin) => args.types().next(),
            _ => None,
        }
    }

    /// A `dyn Future`: a JS promise (ADR 0029).
    pub(super) fn is_dyn_future(&self, ty: Ty<'tcx>) -> bool {
        matches!(ty.kind(), ty::Dynamic(traits, ..)
            if traits.principal_def_id().is_some_and(|t| self.tcx.is_lang_item(t, LangItem::Future)))
    }

    /// A future whose `Future` is the crate's own, or a reference, `Box` or
    /// `Pin` of one: an impl rust-js never lowers (ADR 0425), so no JS
    /// promise is a value of it, and what would await one is refused.
    pub(super) fn unlowered_future(&self, ty: Ty<'tcx>) -> bool {
        let ty = self.future_pointee(ty);
        let Some(poll) = self.tcx.lang_items().get(LangItem::FuturePoll) else {
            return false;
        };
        let args = self.tcx.mk_args(&[ty.into()]);
        matches!(super::items::resolve_instance_in(self.tcx, self.typing_env, poll, args),
            Ok(Some(instance)) if self.tcx.trait_impl_of_assoc(instance.def_id()).is_some_and(|imp| self.is_rust_trait(imp)))
    }

    /// What a future, or a reference, `Box` or `Pin` of one, is of.
    fn future_pointee(&self, mut ty: Ty<'tcx>) -> Ty<'tcx> {
        while let Some(inner) = self.pinned(ty).or_else(|| ty.builtin_deref(true)) {
            ty = inner;
        }
        ty
    }

    /// The `unlowered_future` that unsizing `from` to `to` makes a `dyn
    /// Future` of, as `Box::pin(f)` as a `Pin<Box<dyn Future>>` does.
    pub(super) fn unsized_unlowered_future(&self, from: Ty<'tcx>, to: Ty<'tcx>) -> Option<Ty<'tcx>> {
        (self.is_dyn_future(self.future_pointee(to)) && self.unlowered_future(from)).then(|| self.future_pointee(from))
    }

    /// The `unlowered_future` that calling `def_id` of `args` asks a
    /// `Future` or an `IntoFuture` of, among its bounds: `.await`'s
    /// `into_future(x)`, or a generic function's `F: Future`.
    pub(super) fn awaited_unlowered_future(
        &self,
        (def_id, args): (DefId, ty::GenericArgsRef<'tcx>),
    ) -> Option<Ty<'tcx>> {
        self.tcx
            .clauses_of(def_id)
            .instantiate(self.tcx, args)
            .into_iter()
            .find_map(|(clause, _)| match clause.skip_normalization().kind().skip_binder() {
                ty::ClauseKind::Trait(bound)
                    if self.tcx.is_lang_item(bound.def_id(), LangItem::Future)
                        || self
                            .tcx
                            .is_diagnostic_item(Symbol::intern("IntoFuture"), bound.def_id()) =>
                {
                    let ty = bound.self_ty();
                    self.unlowered_future(ty).then(|| self.future_pointee(ty))
                }
                _ => None,
            })
    }

    pub(super) fn is_std_wrapper(&self, ty: Ty<'tcx>) -> bool {
        ty.is_box()
            || self.pinned(ty).is_some()
            || self.uninit_of(ty).is_some()
            || self.is_lang_adt(ty, LangItem::String)
            // A `Formatter` is its text, a string (ADR 0054): a `&mut` to
            // one a cell, as a `String`'s is (ADR 0461).
            || self.is_formatter_type(ty)
            || self.is_rc(ty)
            || self.is_guard(ty)
            || [
                StdItem::Cell,
                StdItem::RefCell,
                StdItem::Atomic,
                StdItem::Mutex,
                StdItem::RwLock,
            ]
            .into_iter()
            .any(|item| self.is_std_type(ty, item))
            || self.is_std_type(ty, StdItem::OnceCell)
            || self.is_std_type(ty, StdItem::LazyCell)
            || weak_pointee(self.tcx, ty).is_some()
            || self.is_vec_like(ty)
    }

    /// An `Rc`, or an `Arc`, which on one thread is one: the value it
    /// points at, shared (ADR 0023).
    pub(super) fn is_rc(&self, ty: Ty<'tcx>) -> bool {
        self.is_std_type(ty, StdItem::Rc) || self.is_std_type(ty, StdItem::Arc)
    }

    /// Is std's `AsRef<to>` or `AsMut<to>` of a `from`, through as many
    /// references as it's behind, the same JS value (ADR 0162): text as a
    /// `str`, and a `Vec`, an array or a slice as a slice.
    pub(super) fn views_as_itself(&self, from: Ty<'tcx>, to: Ty<'tcx>) -> bool {
        let from = from.peel_refs();
        let sequence = from.is_array() || from.is_slice() || self.is_vec_like(from);
        (self.is_string_like(from) && to.is_str()) || (sequence && to.is_slice())
    }

    /// Is std's `Borrow<to>` of a `from` the same JS value (ADR 0167): a
    /// value as itself, through a reference, a `Box` or an `Rc` (ADR 0023),
    /// text as a `str`, and a `Vec` or an array as a slice.
    pub(super) fn borrows_as_itself(&self, from: Ty<'tcx>, to: Ty<'tcx>) -> bool {
        let pointee = |mut ty: Ty<'tcx>| loop {
            ty = match ty.kind() {
                ty::Ref(_, inner, _) => *inner,
                ty::Adt(_, args) if ty.is_box() || self.is_rc(ty) || self.pinned(ty).is_some() => args.type_at(0),
                _ => break ty,
            };
        };
        let (from, to) = (pointee(from), pointee(to));
        from == to
            || (self.is_string_like(from) && to.is_str())
            // A path is its text (ADR 0173).
            || ((self.is_string_like(from) || self.is_path_like(from)) && self.is_path_like(to))
            || ((from.is_array() || self.is_std_type(from, StdItem::Vec)) && to.is_slice())
    }

    /// A type whose `Borrow` is the crate's that a std function takes as
    /// what it borrows as (ADR 0167): a map's `get(q)` of its keys, its
    /// `K: Borrow<Q>`, or `[S]::join` of its items, through `[S]: Join`'s
    /// impl's `S: Borrow<str>`. std's JS compares and joins the value itself.
    pub(super) fn borrowed_by_user(&self, def_id: DefId, args: ty::GenericArgsRef<'tcx>) -> Option<Ty<'tcx>> {
        let borrow = std_item(self.tcx, StdItem::Borrow);
        let bounds = |id: DefId, args: ty::GenericArgsRef<'tcx>| {
            let clauses = self.tcx.clauses_of(id).instantiate(self.tcx, args).clauses;
            clauses
                .into_iter()
                .filter_map(|clause| clause.skip_normalization().as_trait_clause())
                .map(|clause| {
                    self.tcx
                        .erase_and_anonymize_regions(self.tcx.instantiate_bound_regions_with_erased(clause).trait_ref)
                })
                .collect::<Vec<_>>()
        };
        let users = |tr: ty::TraitRef<'tcx>| (tr.def_id == borrow && self.is_user_impl(tr)).then(|| tr.self_ty());
        bounds(def_id, args).into_iter().find_map(|tr| {
            users(tr).or_else(|| {
                // Of std's impl, whose JS is std's: the crate's is given its
                // `Borrow`'s dictionary.
                match self.tcx.codegen_select_candidate(self.typing_env.as_query_input(tr)) {
                    Ok(ImplSource::UserDefined(imp))
                        if !self.trait_impls.contains(&imp.impl_def_id) && !self.foreign.has_impl(imp.impl_def_id) =>
                    {
                        bounds(imp.impl_def_id, imp.args).into_iter().find_map(users)
                    }
                    _ => None,
                }
            })
        })
    }

    /// A guard of a `RefCell` or a lock: what it guards (ADR 0025).
    pub(super) fn is_guard(&self, ty: Ty<'tcx>) -> bool {
        [
            StdItem::RefCellRef,
            StdItem::RefCellRefMut,
            StdItem::MutexGuard,
            StdItem::RwLockReadGuard,
            StdItem::RwLockWriteGuard,
        ]
        .into_iter()
        .any(|item| self.is_std_type(ty, item))
    }

    /// Whether calling `def_id`, of `args`, may run code of the crate's, or
    /// ask whether a `RefCell` or a lock is borrowed, which only a borrow
    /// that's held answers (ADR 0328): a function of the crate's, a
    /// closure, a `dyn`'s method, a guard's own, or a thread-local cell's.
    pub(super) fn may_ask_borrows(&self, def_id: DefId, args: ty::GenericArgsRef<'tcx>) -> bool {
        let Ok(Some(instance)) = super::items::resolve_instance_in(self.tcx, self.typing_env, def_id, args) else {
            return true;
        };
        let ty::InstanceKind::Item(id) = instance.def else {
            return true;
        };
        // What makes a JSX element or an object, or casts a value, runs no
        // code: a component given is called when React renders it.
        if super::bindings::is_binding(self.tcx, id)
            && matches!(
                super::bindings::js_form(self.tcx, id),
                super::bindings::JsForm::Jsx(_)
                    | super::bindings::JsForm::Prop(_)
                    | super::bindings::JsForm::Object(_)
                    | super::bindings::JsForm::This
            )
        {
            return false;
        }
        !self.is_std(id) || self.asks_borrows(id)
    }

    /// Whether running a body, a closure's, can't ask whether a cell is
    /// borrowed (ADR 0328): each call is std's, that may not, and it drops
    /// nothing of the crate's.
    pub(super) fn asks_no_borrows(&self, thir: &Thir<'tcx>) -> bool {
        thir.exprs.iter().all(|e| {
            let call = match e.kind {
                ExprKind::Call { fun, .. } => {
                    fn_def(thir[fun].ty).is_some_and(|(id, args)| !self.may_ask_borrows(id, args))
                }
                ExprKind::InlineAsm(_)
                | ExprKind::Yield { .. }
                | ExprKind::Become { .. } => false,
                _ => true,
            };
            call && e.ty.walk().all(|part| {
                part.as_type().is_none_or(|t| {
                    !matches!(t.kind(), ty::Adt(adt, _) if !self.is_std(adt.did()) && t.needs_drop(self.tcx, self.typing_env))
                })
            })
        })
    }

    /// A thread-local `RefCell`'s function, or a guard's own, `Ref::map`
    /// say (ADR 0328).
    fn asks_borrows(&self, id: DefId) -> bool {
        let Some(imp) = self.tcx.impl_of_assoc(id) else {
            return false;
        };
        let owner = self.tcx.type_of(imp).instantiate_identity().skip_normalization();
        match owner.kind() {
            _ if self.is_guard(owner) => !self.tcx.impl_is_of_trait(imp),
            ty::Adt(_, args) if self.is_std_type(owner, StdItem::LocalKey) => {
                args.types().next().is_some_and(|held| self.is_borrowed_cell(held))
            }
            _ => false,
        }
    }

    /// A `RefCell`, a `Mutex` or an `RwLock`: what a guard borrows (ADR 0328).
    pub(super) fn is_borrowed_cell(&self, ty: Ty<'tcx>) -> bool {
        [StdItem::RefCell, StdItem::Mutex, StdItem::RwLock]
            .into_iter()
            .any(|item| self.is_std_type(ty, item))
    }

    /// A type made of std's types alone, and no cell: no impl of the crate's
    /// can be for it, nor one that borrows (ADR 0328).
    pub(super) fn std_alone(&self, ty: Ty<'tcx>) -> bool {
        ty.walk().all(|part| {
            part.as_type().is_none_or(|t| match t.kind() {
                ty::Adt(adt, _) => self.is_std(adt.did()) && !self.is_borrowed_cell(t),
                ty::Bool
                | ty::Char
                | ty::Int(_)
                | ty::Uint(_)
                | ty::Float(_)
                | ty::Str
                | ty::Never
                | ty::Array(..)
                | ty::Slice(_)
                | ty::Ref(..)
                | ty::Tuple(_) => true,
                _ => super::pointers::raw_pointee(self.tcx, t).is_some(),
            })
        })
    }

    pub(super) fn is_std(&self, id: DefId) -> bool {
        is_std_item(self.tcx, id)
    }

    /// Is `id` the standard library's: std, core, alloc, or a crate rustc's
    /// sysroot holds beside them, as `hashbrown`, which std's maps are made of?
    pub(super) fn in_sysroot(&self, id: DefId) -> bool {
        self.is_std(id) || {
            let sysroots: Vec<&std::path::Path> = self.tcx.sess.opts.sysroot.all_paths().collect();
            self.tcx
                .used_crate_source(id.krate)
                .paths()
                .any(|path| sysroots.iter().any(|sysroot| path.starts_with(sysroot)))
        }
    }

    pub(super) fn is_derived_impl(&self, trait_id: DefId, ty: Ty<'tcx>) -> bool {
        let tr = ty::TraitRef::new_from_args(self.tcx, trait_id, self.args_of(trait_id, ty));
        let tr = self.tcx.erase_and_anonymize_regions(tr);
        matches!(self.tcx.codegen_select_candidate(self.typing_env.as_query_input(tr)),
            Ok(ImplSource::UserDefined(imp)) if self.trait_impls.contains(&imp.impl_def_id)
                && known_derive(self.tcx, imp.impl_def_id))
    }

    /// Is `ty`'s impl of `trait_id` a `#[derive]`d one, whether or not
    /// rust-js lowers it: a derived `PartialEq` it never does.
    pub(super) fn derives(&self, trait_id: DefId, ty: Ty<'tcx>) -> bool {
        let tr = ty::TraitRef::new_from_args(self.tcx, trait_id, self.args_of(trait_id, ty));
        let tr = self.tcx.erase_and_anonymize_regions(tr);
        matches!(self.tcx.codegen_select_candidate(self.typing_env.as_query_input(tr)),
            Ok(ImplSource::UserDefined(imp)) if known_derive(self.tcx, imp.impl_def_id))
    }
}

pub(super) fn serde_trait(tcx: TyCtxt<'_>, trait_id: DefId) -> Option<bool> {
    if !matches!(tcx.crate_name(trait_id.krate).as_str(), "serde" | "serde_core") {
        return None;
    }
    match tcx.item_name(trait_id).as_str() {
        "Serialize" => Some(true),
        "Deserialize" | "DeserializeOwned" => Some(false),
        _ => None,
    }
}

/// A derive whose meaning rust-js has itself, never its body's: std's own,
/// `#[derive(Clone)]` and the rest. Another crate's, strum's or thiserror's,
/// writes code like any of the crate's, `#[automatically_derived]` though it
/// is (ADR 0186); serde's is left out where it's read, as its codecs are
/// rust-js's (ADR 0077). rustc tells which macro wrote only this crate's,
/// the one compiled here: another's, core's `IntErrorKind`'s `Debug`, is as
/// it says.
pub(crate) fn known_derive(tcx: TyCtxt<'_>, id: DefId) -> bool {
    match id.is_local() {
        true => tcx.is_builtin_derived(id),
        false => tcx.is_automatically_derived(id),
    }
}

pub(super) fn from_serde_derive(tcx: TyCtxt<'_>, def_id: LocalDefId) -> bool {
    let mut item = Some(def_id);
    while let Some(id) = item {
        let mut ctxt = tcx.def_span(id).ctxt();
        while !ctxt.is_root() {
            let expansion = ctxt.outer_expn_data();
            // By the macro, not its name: `serde::Deserialize` and an alias are
            // named as they're written.
            if let ExpnKind::Macro(MacroKind::Derive, _) = expansion.kind
                && expansion
                    .macro_def_id
                    .is_some_and(|id| tcx.crate_name(id.krate).as_str() == "serde_derive")
            {
                return true;
            }
            ctxt = expansion.call_site.ctxt();
        }
        item = tcx.opt_local_parent(id);
    }
    false
}

pub(super) fn serde_impl(tcx: TyCtxt<'_>, id: DefId) -> Option<bool> {
    if !matches!(tcx.def_kind(id), DefKind::Impl { of_trait: true }) {
        return None;
    }
    let tr = tcx.impl_trait_ref(id).instantiate_identity().skip_normalization();
    let own = matches!(tr.self_ty().kind(), ty::Adt(adt, _)
        if adt.did().as_local().is_some_and(|local| !from_serde_derive(tcx, local)));
    serde_trait(tcx, tr.def_id).filter(|_| own)
}

/// Whether serde impl `id` is serde's derive's, a codec rust-js writes by
/// its `#[serde]` attributes (ADR 0077), not a hand-written one, whose body
/// is the crate's (ADR 0459). A library's, by its `#[automatically_derived]`.
pub(super) fn serde_derived(tcx: TyCtxt<'_>, id: DefId) -> bool {
    match id.as_local() {
        Some(local) => from_serde_derive(tcx, local),
        None => tcx.is_automatically_derived(id),
    }
}

/// Is `id` core's `Sum` or `Product`, of `sum()` and `product()`, which
/// have no diagnostic items?
pub(crate) fn is_sum_or_product(tcx: TyCtxt<'_>, id: DefId) -> bool {
    accumulation(tcx, id).is_some()
}

/// The `Sum` or the `Product` `Iterator::sum` or `product`, `method`, bounds
/// what it makes by, `S: Sum<Self::Item>`.
pub(crate) fn accumulation_of(tcx: TyCtxt<'_>, method: DefId) -> Option<DefId> {
    (tcx.clauses_of(method).clauses.iter())
        .filter_map(|(clause, _)| clause.as_trait_clause())
        .map(|bound| bound.def_id())
        .find(|&tr| accumulation(tcx, tr).is_some())
}

/// std's `Sum` or `Product`, by its method's name, `sum` or `product`.
pub(crate) fn accumulation(tcx: TyCtxt<'_>, id: DefId) -> Option<&'static str> {
    if tcx.crate_name(id.krate) != sym::core || tcx.def_kind(id) != DefKind::Trait {
        return None;
    }
    match tcx.item_name(id).as_str() {
        "Sum" => Some("sum"),
        "Product" => Some("product"),
        _ => None,
    }
}

/// The `fmt` traits of a placeholder's other than `{}` and `{:?}`: `{:x}`'s
/// `LowerHex` and the like, and `{:p}`'s `Pointer`, most with no diagnostic
/// item (ADR 0165).
const OTHER_FMT_TRAITS: [&str; 7] = [
    "LowerHex", "UpperHex", "Octal", "Binary", "LowerExp", "UpperExp", "Pointer",
];

/// Is `id` one of core's `OTHER_FMT_TRAITS`?
pub(crate) fn is_other_fmt_trait(tcx: TyCtxt<'_>, id: DefId) -> bool {
    tcx.crate_name(id.krate) == sym::core
        && tcx.def_kind(id) == DefKind::Trait
        && OTHER_FMT_TRAITS.contains(&tcx.item_name(id).as_str())
        && std_path(tcx, id).contains("fmt::")
}

/// Is `id` core's `DoubleEndedIterator`?
pub(crate) fn is_double_ended(tcx: TyCtxt<'_>, id: DefId) -> bool {
    is_iterator_extension(tcx, id) && tcx.item_name(id).as_str() == "DoubleEndedIterator"
}

/// Is `id` core's `ExactSizeIterator`, whose `len()` a JS iterator given for
/// one knows (ADR 0456)?
pub(crate) fn is_exact_size(tcx: TyCtxt<'_>, id: DefId) -> bool {
    is_iterator_extension(tcx, id) && tcx.item_name(id).as_str() == "ExactSizeIterator"
}

/// Is `id` core's `DoubleEndedIterator` or `ExactSizeIterator`, which have
/// no diagnostic items?
pub(crate) fn is_iterator_extension(tcx: TyCtxt<'_>, id: DefId) -> bool {
    tcx.crate_name(id.krate) == sym::core
        && tcx.def_kind(id) == DefKind::Trait
        && [
            Symbol::intern("DoubleEndedIterator"),
            Symbol::intern("ExactSizeIterator"),
        ]
        .contains(&tcx.item_name(id))
}

/// Whether a bound of `trait_id` on `self_ty` is given a dictionary as
/// `IntoIterator`'s: one on a reference to a type only a caller knows,
/// `for<'a> &'a mut C: IntoIterator`, whose generic code keeps `C` and
/// iterates it through a `&` or a `&mut` with what its caller gives (ADR
/// 0457). One on a type parameter itself, `I: IntoIterator`, is given the
/// JS iterator (ADR 0061).
pub(crate) fn into_iter_by_dictionary(tcx: TyCtxt<'_>, trait_id: DefId, self_ty: Ty<'_>) -> bool {
    tcx.is_diagnostic_item(sym::IntoIterator, trait_id)
        && matches!(self_ty.kind(), ty::Ref(_, inner, _) if matches!(inner.kind(), ty::Param(_) | ty::Alias(..)))
}

pub(super) fn is_extend(tcx: rustc_middle::ty::TyCtxt<'_>, trait_id: rustc_span::def_id::DefId) -> bool {
    tcx.crate_name(trait_id.krate) == rustc_span::sym::core && tcx.item_name(trait_id) == Symbol::intern("Extend")
}

/// How an explicitly fallible JavaScript binding delivers its result.
pub(super) enum Catching {
    Direct,
    Result,
    PromiseResult,
}

#[derive(Clone, Copy)]
pub(super) enum OrderingCall {
    Compare,
    Lt,
    Le,
    Gt,
    Ge,
    Max,
    Min,
}

impl<'a, 'tcx> Recognition<'a, 'tcx> {
    /// Of the call's own types, as `json::stringify_with` of an `Unknown`
    /// may throw where one of a `JsonText` can't (ADR 0310).
    /// `Display::fmt`, whose impl says how `format!` shows a value.
    pub(super) fn display_fmt(&self) -> Option<DefId> {
        let display = self.tcx.get_diagnostic_item(sym::Display)?;
        self.tcx.associated_item_def_ids(display).first().copied()
    }

    pub(super) fn catching(&self, id: DefId, args: ty::GenericArgsRef<'tcx>) -> Catching {
        let output = self
            .tcx
            .fn_sig(id)
            .instantiate(self.tcx, args)
            .skip_normalization()
            .skip_binder()
            .output();
        let output = self
            .tcx
            .try_normalize_erasing_regions(self.typing_env, ty::Unnormalized::new_wip(output))
            .unwrap_or(output);
        if self.is_std_type(output, StdItem::Result) {
            Catching::Result
        } else if matches!(output.kind(), ty::Adt(adt, args) if self.is_js_object(output)
            && self.tcx.item_name(adt.did()).as_str() == "Promise"
            && args.types().next().is_some_and(|t| self.is_std_type(t, StdItem::Result)))
        {
            Catching::PromiseResult
        } else {
            Catching::Direct
        }
    }

    pub(super) fn ordering_call(&self, id: DefId, tr: ty::TraitRef<'tcx>) -> Option<(OrderingCall, bool)> {
        let partial = self.tcx.is_lang_item(tr.def_id, LangItem::PartialOrd);
        if !partial && !self.tcx.is_diagnostic_item(sym::Ord, tr.def_id) {
            return None;
        }
        let name = self.tcx.item_name(id);
        // A number's `max` and `cmp` are its own methods'; its `<` is JS's, as
        // a copied default's `self < Self::zero()` of an `f64` calls it.
        if Num::of(tr.self_ty().peel_refs()).is_some()
            && !matches!(name.as_str(), "partial_cmp" | "lt" | "le" | "gt" | "ge")
        {
            return None;
        }
        let call = match name.as_str() {
            "lt" => OrderingCall::Lt,
            "le" => OrderingCall::Le,
            "gt" => OrderingCall::Gt,
            "ge" => OrderingCall::Ge,
            "cmp" | "partial_cmp" => OrderingCall::Compare,
            "max" => OrderingCall::Max,
            "min" => OrderingCall::Min,
            _ => return None,
        };
        Some((call, partial))
    }
}

/// Is `id` a trait whose impls are passed as dictionaries (ADR 0049)? One of
/// the crate's own, or a library's (ADR 0100), which its consumers must pass
/// as it takes them, and the std traits rust-js calls.
pub(super) fn operational(tcx: TyCtxt<'_>, foreign: &super::library::Foreign<'_, '_>, id: DefId) -> bool {
    id.is_local()
        || foreign.in_library(id)
        || tcx.is_lang_item(id, LangItem::Copy)
        || tcx.is_lang_item(id, LangItem::Clone)
        || tcx.is_lang_item(id, LangItem::PartialEq)
        || tcx.is_lang_item(id, LangItem::PartialOrd)
        || tcx.is_diagnostic_item(sym::Ord, id)
        || tcx.is_diagnostic_item(Symbol::intern("Display"), id)
        || tcx.is_diagnostic_item(Symbol::intern("Debug"), id)
        || tcx.is_diagnostic_item(Symbol::intern("Default"), id)
        // Std's of a `Display`: `{ to_string }`, what it shows.
        || tcx.is_diagnostic_item(Symbol::intern("ToString"), id)
        // A `dyn Error`'s (ADR 0141), its `Display` and `Debug` its supertraits'.
        || tcx.is_diagnostic_item(Symbol::intern("Error"), id)
        // A `dyn Any`'s, its type's key (ADR 0331).
        || tcx.is_diagnostic_item(Symbol::intern("Any"), id)
        // Its evidence is the writer or reader itself (ADR 0081).
        || serde_trait(tcx, id).is_some()
        // `parse` of a `T: FromStr`: its dictionary's `from_str` (ADR 0161).
        || is_from_str(tcx, id)
        // `s.as_ref()` of an `S: AsRef<str>`: its dictionary's `as_ref`, and
        // `s.as_mut()` of an `S: AsMut<str>` its `as_mut` (ADR 0162).
        || is_std_def(tcx, id, StdItem::AsRef)
        || is_std_def(tcx, id, StdItem::AsMut)
        // `key.borrow()` of a `K: Borrow<Q>`, as a map's lookup is: its
        // dictionary's `borrow` (ADR 0167); and `v.borrow_mut()` of a `V:
        // BorrowMut<Q>`, its `borrow_mut`, a supertrait's too (ADR 0483).
        || is_std_def(tcx, id, StdItem::Borrow)
        || is_std_def(tcx, id, StdItem::BorrowMut)
        // `{:x}`, `{:e}` and `{:p}` of the crate's types (ADR 0165), and of a
        // `T: LowerHex` or `T: Pointer` (ADR 0174).
        || is_other_fmt_trait(tcx, id)
        // `a + b` of a `T: Add`, and `x.into()` of a `T: Into<U>` (ADR 0108).
        || value_operator(tcx, id).is_some()
        || tcx.is_diagnostic_item(sym::Into, id)
        // `W::from(u)` of a `W: From<U>`: its dictionary's `from` (ADR 0477).
        || tcx.is_diagnostic_item(sym::From, id)
        // A writer of the crate's own, given its text a `write!` at a time
        // (ADR 0166), and generic code that writes to any writer, a `W:
        // fmt::Write`: its dictionary's `write_str`, `write_char` and
        // `write_fmt` (ADR 0180).
        || tcx.is_diagnostic_item(Symbol::intern("FmtWrite"), id)
        // Generic code, or a `dyn`, that writes bytes to any writer, a `W:
        // io::Write`: its dictionary's `write`, `write_all`, `write_fmt` and
        // `flush` (ADR 0392).
        || is_std_def(tcx, id, StdItem::IoWrite)
        // `range.start_bound()` of an `R: RangeBounds<usize>`, arrayvec's
        // `drain`: its dictionary's, a range's bounds as `Bound`s.
        || is_std_def(tcx, id, StdItem::RangeBounds)
        // `v[i]` of an `I: SliceIndex<[T]>`: its dictionary's (ADR 0418).
        || is_std_def(tcx, id, StdItem::SliceIndex)
        // `&*p` of a `P: Deref`: its dictionary's `deref` (ADR 0394), and
        // `&mut *p` of a `P: DerefMut` its `deref_mut`.
        || tcx.is_lang_item(id, LangItem::Deref)
        || tcx.is_lang_item(id, LangItem::DerefMut)
        // `a += b` of a `T: AddAssign`: its dictionary's `add_assign` (ADR 0396).
        || assign_operator(tcx, id).is_some()
        // `c.extend(items)` of a `C: Extend<A>`, and `collect` into a `B:
        // FromIterator<A>`: its dictionary's `extend` or `from_iter` (ADR 0445).
        || is_extend(tcx, id)
        || tcx.is_diagnostic_item(sym::FromIterator, id)
        // `for x in c` of a `for<'a> &'a mut C: IntoIterator`: its
        // dictionary's `into_iter` (ADR 0457).
        || tcx.is_diagnostic_item(sym::IntoIterator, id)
        // `value.hash(state)` of a `T: Hash`, and a hasher's writes of an `H:
        // Hasher`: their dictionaries' `hash`, and `write`, `write_u32`, ...
        // and `finish` (ADR 0463).
        || is_hashing(tcx, id)
        // `items.sum()` into an `S: Sum<A>`, and `product()` into a `P:
        // Product<A>`: its dictionary's `sum` or `product` (ADR 0476), the
        // crate's collection's too (ADR 0160).
        || is_sum_or_product(tcx, id)
}

/// `Hash` or `Hasher`: hashing's traits, whose impls a map never calls
/// (ADR 0463).
pub(crate) fn is_hashing(tcx: TyCtxt<'_>, id: DefId) -> bool {
    is_std_def(tcx, id, StdItem::Hash) || is_hasher(tcx, id)
}

/// `std::hash::Hasher`, which has no diagnostic item (ADR 0463).
pub(crate) fn is_hasher(tcx: TyCtxt<'_>, id: DefId) -> bool {
    std_path(tcx, id) == "std::hash::Hasher"
}

/// An operator that makes a value, `a + b` or `-a`: what a number's is, as
/// the operator of a MIR binary or unary operation.
pub(super) fn value_operator(tcx: TyCtxt<'_>, id: DefId) -> Option<Result<BinOp, UnOp>> {
    [
        (LangItem::Add, Ok(BinOp::Add)),
        (LangItem::Sub, Ok(BinOp::Sub)),
        (LangItem::Mul, Ok(BinOp::Mul)),
        (LangItem::Div, Ok(BinOp::Div)),
        (LangItem::Rem, Ok(BinOp::Rem)),
        (LangItem::BitAnd, Ok(BinOp::BitAnd)),
        (LangItem::BitOr, Ok(BinOp::BitOr)),
        (LangItem::BitXor, Ok(BinOp::BitXor)),
        (LangItem::Shl, Ok(BinOp::Shl)),
        (LangItem::Shr, Ok(BinOp::Shr)),
        (LangItem::Neg, Err(UnOp::Neg)),
        (LangItem::Not, Err(UnOp::Not)),
    ]
    .into_iter()
    .find_map(|(item, op)| tcx.is_lang_item(id, item).then_some(op))
}

pub(super) fn implementable(tcx: TyCtxt<'_>, foreign: &super::library::Foreign<'_, '_>, id: DefId) -> bool {
    operational(tcx, foreign, id)
        // A hand-written `Deserialize`'s visitor, its methods the reader's
        // visitor's (ADR 0459).
        || serde_visitor(tcx, id)
        || tcx.is_diagnostic_item(sym::TryFrom, id)
        // Never lowered (ADRs 0168, 0425).
        || unlowered_trait(tcx, id)
        // An iterator of the crate's from both ends, and of a known length:
        // what `rev()`, `next_back()` and `len()` call (ADR 0164).
        || is_iterator_extension(tcx, id)
        || tcx.is_diagnostic_item(sym::Eq, id)
        || tcx.is_diagnostic_item(sym::Iterator, id)
        || is_operator(tcx, id)
        // A trait of no items: its impl says what the type may be used for,
        // and runs nothing. An auto trait, as `Send`, `Sync`, `Unpin` or
        // `UnwindSafe`; `FusedIterator`, a promise that `next` stays `None`,
        // which `fuse()` is without (ADR 0055); a binding crate's marker of
        // what its bindings take, react's `Key` of a list's key. Or of types
        // only, which rustc works out: react's `Lifetimes`, a props type's
        // `'static` form (ADR 0265).
        || (tcx.associated_item_def_ids(id).iter()).all(|&item| matches!(tcx.def_kind(item), DefKind::AssocTy))
        // Run where rustc drops a value (ADR 0098).
        || tcx.is_lang_item(id, LangItem::Drop)
}

/// A std trait whose impls, the crate's own, rust-js allows and never
/// lowers: what would call one is refused where it would. `Future`, as
/// `.await` is JS's of a promise (ADR 0029); and `io::Read`, `BufRead` and
/// `Seek`, which JS has no stream of (ADR 0425). The one list.
pub(crate) fn unlowered_trait(tcx: TyCtxt<'_>, id: DefId) -> bool {
    tcx.is_lang_item(id, LangItem::Future)
        || matches!(
            std_path(tcx, id).as_str(),
            "std::io::Read" | "std::io::BufRead" | "std::io::Seek"
        )
}

/// An impl of one of `unlowered_trait`'s, derived or the crate's own.
pub(crate) fn is_unlowered_impl(tcx: TyCtxt<'_>, id: DefId) -> bool {
    matches!(tcx.def_kind(id), DefKind::Impl { of_trait: true })
        && unlowered_trait(
            tcx,
            tcx.impl_trait_ref(id)
                .instantiate_identity()
                .skip_normalization()
                .def_id,
        )
}

/// What a conversion converts, into what, of its trait's method's `args`:
/// `From`'s and `TryFrom`'s from their argument into their type, `Into`'s
/// and `TryInto`'s from their type into their argument. The one answer of
/// which side is which.
pub(crate) fn conversion_sides<'tcx>(
    tcx: TyCtxt<'tcx>,
    trait_: DefId,
    args: ty::GenericArgsRef<'tcx>,
) -> Option<(Ty<'tcx>, Ty<'tcx>)> {
    let (this, other) = (args.types().next()?, args.types().nth(1)?);
    let is = |name: Symbol| tcx.is_diagnostic_item(name, trait_);
    match () {
        _ if is(sym::From) || is(sym::TryFrom) => Some((other, this)),
        _ if is(sym::Into) || is(sym::TryInto) => Some((this, other)),
        _ => None,
    }
}

/// What an `Rc` or an `Arc`, `to`, a conversion makes of `from` points at,
/// where it's `Rc::new`'s of it: of the value, of a `str` its text, a
/// `&str`, a `String` or a `Box<str>`, of a `[T]` a `Vec<T>`'s or a
/// `Box<[T]>`'s items, moved.
pub(crate) fn rc_made_of<'tcx>(tcx: TyCtxt<'tcx>, from: Ty<'tcx>, to: Ty<'tcx>) -> Option<Ty<'tcx>> {
    let pointee = rc_pointee(tcx, to)?;
    let erased = |ty: Ty<'tcx>| tcx.erase_and_anonymize_regions(ty);
    let text = |ty: Ty<'tcx>| {
        ty.peel_refs().is_str()
            || matches!(ty.kind(), ty::Adt(adt, _) if tcx.is_lang_item(adt.did(), LangItem::String))
            || ty.boxed_ty().is_some_and(|inner| inner.is_str())
    };
    let items = |item: Ty<'tcx>| match from.kind() {
        ty::Adt(adt, args) if is_std_def(tcx, adt.did(), StdItem::Vec) => erased(args.type_at(0)) == erased(item),
        _ => from
            .boxed_ty()
            .is_some_and(|inner| matches!(inner.kind(), ty::Slice(i) if erased(*i) == erased(item))),
    };
    let made = match pointee.kind() {
        _ if erased(pointee) == erased(from) => true,
        ty::Str => text(from) && !matches!(from.kind(), ty::Ref(_, _, Mutability::Mut)),
        ty::Slice(item) => items(*item),
        _ => false,
    };
    made.then_some(pointee)
}

/// What a conversion makes, a `From` or an `Into`: the trait's method's
/// target, of its `args`, or the impl's own type of a `from` a call resolves
/// to. An untagged enum's is the value itself (ADR 0214).
pub(crate) fn conversion_target<'tcx>(
    tcx: TyCtxt<'tcx>,
    def_id: DefId,
    args: ty::GenericArgsRef<'tcx>,
) -> Option<Ty<'tcx>> {
    if let Some(imp) = tcx.trait_impl_of_assoc(def_id) {
        let trait_ref = tcx.impl_trait_ref(imp).instantiate_identity().skip_normalization();
        return tcx
            .is_diagnostic_item(sym::From, trait_ref.def_id)
            .then(|| trait_ref.self_ty());
    }
    let trait_ = tcx.trait_of_assoc(def_id)?;
    if tcx.is_diagnostic_item(sym::From, trait_) {
        return Some(args.type_at(0));
    }
    tcx.is_diagnostic_item(sym::Into, trait_).then(|| args.type_at(1))
}

/// A JS object type's `Deref` to the class it extends, `TypeError`'s to
/// `Error`: never lowered, as a call of it is the object itself (ADR 0024),
/// and its body, a cast of a pointer, means nothing in JS.
pub(crate) fn is_js_object_deref(tcx: TyCtxt<'_>, id: DefId) -> bool {
    matches!(tcx.def_kind(id), DefKind::Impl { of_trait: true })
        && [LangItem::Deref, LangItem::DerefMut].into_iter().any(|item| {
            tcx.is_lang_item(
                tcx.impl_trait_ref(id)
                    .instantiate_identity()
                    .skip_normalization()
                    .def_id,
                item,
            )
        })
        && matches!(tcx.type_of(id).instantiate_identity().skip_normalization().kind(),
        ty::Adt(adt, _) if adt.is_struct()
            && adt.non_enum_variant().fields.iter().next().is_some_and(|first| {
                matches!(tcx.type_of(first.did).instantiate_identity().skip_normalization().kind(),
                    ty::Adt(marker, marked) if marker.is_phantom_data()
                        && marked.types().next().is_some_and(|t| match t.kind() {
                            ty::Foreign(_) => true,
                            ty::Adt(object, _) => tcx
                                .get_attrs_by_path(object.did(), &[Symbol::intern("rust_js"), Symbol::intern("js_object")])
                                .next()
                                .is_some(),
                            _ => false,
                        }))
            }))
}

/// Where a `Cell` or a `RefCell` keeps what it holds, its `UnsafeCell`
/// field `value`: std's layout, of which a constant one is made.
pub(crate) fn cell_value_field(adt: ty::AdtDef<'_>) -> Option<usize> {
    adt.non_enum_variant().fields.iter().position(|f| f.name == sym::value)
}

/// An impl of `Deref`'s `Target`, the type it derefs to.
pub(crate) fn deref_target(tcx: TyCtxt<'_>, imp: DefId) -> Option<DefId> {
    let target = tcx
        .associated_items(imp)
        .filter_by_name_unhygienic(sym::Target)
        .next()?;
    Some(target.def_id)
}

/// A std item's path as std names it, `std::str::Chars`: a `#![no_std]`
/// crate's rustc names it `core::str::Chars`, or `alloc::..`, which every
/// path compared here would miss.
/// std's functions of the process a program runs in, which Node has and a
/// browser hasn't (ADR 0391), each by its path: the one list of what works
/// in Node only. Each is a helper of the runtime's `process.js`.
const NODE_ONLY: &[(&str, ProcessOp)] = &[
    ("std::env::args", ProcessOp::Args),
    ("std::env::var", ProcessOp::Var),
    ("std::env::var_os", ProcessOp::VarOs),
    ("std::env::vars", ProcessOp::Vars),
    ("std::env::vars_os", ProcessOp::VarsOs),
    ("std::process::exit", ProcessOp::Exit),
];

/// Whether `def_id`, a function, is given its caller's location: its own
/// `#[track_caller]`, or its trait's declaration's, which rustc's
/// `codegen_fn_attrs` says of both (ADR 0427).
pub(crate) fn tracks_caller(tcx: TyCtxt<'_>, def_id: DefId) -> bool {
    matches!(tcx.def_kind(def_id), DefKind::Fn | DefKind::AssocFn)
        && tcx
            .codegen_fn_attrs(def_id)
            .flags
            .contains(rustc_middle::middle::codegen_fn_attrs::CodegenFnAttrFlags::TRACK_CALLER)
}

/// Whether `def_id` is a C-variadic function, `extern "C" fn f(mut args:
/// ...)`, whose last parameter gathers what it's given past the rest
/// (ADR 0428).
pub(crate) fn is_c_variadic(tcx: TyCtxt<'_>, def_id: DefId) -> bool {
    matches!(tcx.def_kind(def_id), DefKind::Fn | DefKind::AssocFn)
        && tcx.fn_sig(def_id).skip_binder().skip_binder().c_variadic()
}

/// A C-variadic function's `VaList`, the arguments it hasn't read yet: an
/// array of them (ADR 0428).
pub(crate) fn is_va_list(tcx: TyCtxt<'_>, ty: Ty<'_>) -> bool {
    matches!(ty.kind(), ty::Adt(list, _) if std_path(tcx, list.did()) == "std::ffi::VaList")
}

/// Which of `panic::Location`'s functions `id` is, if it's one (ADR 0427).
pub(crate) fn location_of(tcx: TyCtxt<'_>, id: DefId) -> Option<super::std_types::location::LocationOp> {
    super::std_types::location::location_op(&std_path(tcx, id))
}

/// Which of std's functions that work in Node only `id` is, if it's one.
pub(crate) fn node_only(tcx: TyCtxt<'_>, id: DefId) -> Option<ProcessOp> {
    let path = std_path(tcx, id);
    NODE_ONLY.iter().find(|(of, _)| *of == path).map(|&(_, op)| op)
}

/// `mem::Discriminant<T>`, its variant's discriminant, a number (ADR 0393).
pub(crate) fn is_discriminant_type<'tcx>(tcx: TyCtxt<'tcx>, ty: Ty<'tcx>) -> bool {
    matches!(ty.kind(), ty::Adt(adt, _) if std_path(tcx, adt.did()) == "std::mem::Discriminant")
}

/// std's functions of addressable memory, which JavaScript has none of
/// (ADR 0390): `std::ptr`'s and a raw pointer's own, and what makes a value
/// of bytes, `transmute` and `zeroed`.
pub(crate) fn addresses_memory(tcx: TyCtxt<'_>, id: DefId) -> bool {
    let path = std_path(tcx, id);
    path.starts_with("std::ptr::") || matches!(path.as_str(), "std::mem::transmute_copy" | "std::mem::uninitialized")
}

/// A type of addressable memory, or with one in it (ADR 0390): a raw
/// pointer, a union, whose fields share their bytes, or std's `NonNull`;
/// in what a crate's own type holds too, but not in std's, whose private
/// fields a `Vec`'s pointer is among.
pub(crate) fn is_memory_type<'tcx>(tcx: TyCtxt<'tcx>, ty: Ty<'tcx>) -> bool {
    fn walk<'tcx>(tcx: TyCtxt<'tcx>, ty: Ty<'tcx>, seen: &mut Vec<Ty<'tcx>>) -> bool {
        if seen.contains(&ty) {
            return false;
        }
        seen.push(ty);
        ty.walk().filter_map(|arg| arg.as_type()).any(|part| match part.kind() {
            _ if super::pointers::raw_pointee(tcx, part).is_some() => true,
            ty::Adt(adt, _) if adt.did().is_local() && part != ty => walk(tcx, part, seen),
            ty::Adt(adt, args) if adt.did().is_local() => adt
                .all_fields()
                .any(|field| walk(tcx, field.ty(tcx, args).skip_normalization(), seen)),
            _ => false,
        })
    }
    walk(tcx, ty, &mut Vec::new())
}

/// Whether `id` is std's: of `std`, `core` or `alloc`.
pub(crate) fn is_std_crate(tcx: TyCtxt<'_>, id: DefId) -> bool {
    !id.is_local() && matches!(tcx.crate_name(id.krate).as_str(), "core" | "alloc" | "std")
}

fn std_path(tcx: TyCtxt<'_>, id: DefId) -> String {
    let mut path = tcx.def_path_str(id);
    // Named through a crate's own name for std's, hashbrown's `extern crate
    // alloc as stdalloc`: std's own path, `alloc::alloc::alloc`.
    if is_std_crate(tcx, id) && !matches!(path.split("::").next(), Some("std" | "core" | "alloc")) {
        path = ty::print::with_no_visible_paths!(tcx.def_path_str(id));
    }
    match path.split_once("::") {
        Some(("core" | "alloc", rest)) if !id.is_local() => format!("std::{rest}"),
        _ => path,
    }
}

/// `std::num::NonZero`, which is its number (ADR 0177). Asked where no
/// `TyCtxt` is passed, by `Num::of`, of rustc's own for the thread.
pub(crate) fn is_non_zero(id: DefId) -> bool {
    ty::tls::with_opt(|tcx| tcx.is_some_and(|tcx| tcx.is_diagnostic_item(Symbol::intern("NonZero"), id)))
}

/// std's `Duration`, which is its nanoseconds (ADR 0188).
pub(crate) fn is_duration(id: DefId) -> bool {
    ty::tls::with_opt(|tcx| tcx.is_some_and(|tcx| tcx.is_diagnostic_item(Symbol::intern("Duration"), id)))
}

/// A `Duration` type (ADR 0188).
pub(crate) fn is_duration_ty(ty: Ty<'_>) -> bool {
    matches!(ty.kind(), ty::Adt(adt, _) if is_duration(adt.did()))
}

/// A clock's method `name` (ADR 0447), of its nanoseconds.
fn clock_method(clock: ClockType, name: &str) -> Option<Std> {
    let op = match (clock, name) {
        (ClockType::Error, "duration") => ClockOp::ErrorDuration,
        (ClockType::Error, _) => return None,
        (_, "now") => ClockOp::Now,
        (_, "duration_since" | "saturating_duration_since") => ClockOp::Since { checked: false },
        (_, "checked_duration_since") => ClockOp::Since { checked: true },
        (_, "elapsed") => ClockOp::Elapsed,
        (_, "checked_add") => ClockOp::Shift { sub: false },
        (_, "checked_sub") => ClockOp::Shift { sub: true },
        _ => return None,
    };
    Some(Std::Clock {
        op,
        instant: clock == ClockType::Instant,
    })
}

/// Whether `ty`'s arithmetic is its number's, `a += b` its JS number's
/// `+`, checked as Rust's is: not a `Duration`'s (ADR 0188) or a clock's
/// (ADR 0447), numbers whose `+` std writes, which is compiled.
pub(crate) fn has_number_arithmetic(ty: Ty<'_>) -> bool {
    Num::of(ty).is_some() && !is_duration_ty(ty) && clock_type_of(ty).is_none()
}

/// std's `PoisonError`, a lock's `Err`, which has no diagnostic item.
fn is_poison_error(ty: Ty<'_>) -> bool {
    ty::tls::with_opt(|tcx| {
        tcx.is_some_and(|tcx| {
            matches!(ty.kind(), ty::Adt(adt, _)
                if tcx.crate_name(adt.did().krate) == sym::std && std_path(tcx, adt.did()) == "std::sync::PoisonError")
        })
    })
}

/// What std's clock has of `id` (ADR 0447): `SystemTime`, `Instant`, or
/// `SystemTimeError`.
#[derive(Clone, Copy, PartialEq)]
pub(crate) enum ClockType {
    System,
    Instant,
    Error,
}

pub(crate) fn clock_type(id: DefId) -> Option<ClockType> {
    ty::tls::with_opt(|tcx| {
        let tcx = tcx?;
        if tcx.is_diagnostic_item(Symbol::intern("Instant"), id) {
            return Some(ClockType::Instant);
        }
        if tcx.crate_name(id.krate) != sym::std {
            return None;
        }
        match std_path(tcx, id).as_str() {
            "std::time::SystemTime" => Some(ClockType::System),
            "std::time::SystemTimeError" => Some(ClockType::Error),
            _ => None,
        }
    })
}

/// A clock's type, `SystemTime`, `Instant` or `SystemTimeError` (ADR 0447).
pub(crate) fn clock_type_of(ty: Ty<'_>) -> Option<ClockType> {
    match ty.kind() {
        ty::Adt(adt, _) => clock_type(adt.did()),
        _ => None,
    }
}

/// core's `TryFromSliceError` (ADR 0189).
pub(crate) fn is_try_from_slice_error(tcx: TyCtxt<'_>, ty: Ty<'_>) -> bool {
    matches!(ty.kind(), ty::Adt(adt, _) if tcx.crate_name(adt.did().krate) == sym::core
        && tcx.item_name(adt.did()).as_str() == "TryFromSliceError")
}

/// `String::from_utf16`'s error, of a lone surrogate, the one kind it makes
/// (ADR 0323).
pub(crate) fn is_from_utf16_error(tcx: TyCtxt<'_>, ty: Ty<'_>) -> bool {
    matches!(ty.kind(), ty::Adt(adt, _) if tcx.crate_name(adt.did().krate) == sym::alloc
        && tcx.item_name(adt.did()).as_str() == "FromUtf16Error")
}

/// `char::decode_utf16`'s error, its unpaired surrogate's code (ADR 0327).
pub(crate) fn is_decode_utf16_error(tcx: TyCtxt<'_>, ty: Ty<'_>) -> bool {
    matches!(ty.kind(), ty::Adt(adt, _) if tcx.crate_name(adt.did().krate) == sym::core
        && tcx.item_name(adt.did()).as_str() == "DecodeUtf16Error")
}

/// A `char`'s or a `str`'s `escape_default()`, `escape_debug()` or
/// `escape_unicode()`, and a byte slice's `escape_ascii()`, its text (ADR
/// 0327).
pub(crate) fn is_text_escape(tcx: TyCtxt<'_>, ty: Ty<'_>) -> bool {
    matches!(ty.kind(), ty::Adt(adt, _) if tcx.crate_name(adt.did().krate) == sym::core
        && (matches!(tcx.item_name(adt.did()).as_str(), "EscapeDefault" | "EscapeDebug" | "EscapeUnicode")
            && matches!(std_path(tcx, adt.did()).as_str(), p if p.starts_with("std::char::") || p.starts_with("std::str::"))
            // A byte slice's `escape_ascii()` (ADR 0341).
            || std_path(tcx, adt.did()) == "std::slice::EscapeAscii"))
}

/// A `char`'s `to_uppercase()` or `to_lowercase()`: the `char`s of its
/// mapping, an array (ADR 0070).
pub(crate) fn is_case_mapping(tcx: TyCtxt<'_>, ty: Ty<'_>) -> bool {
    matches!(ty.kind(), ty::Adt(adt, _) if tcx.crate_name(adt.did().krate) == sym::core
        && matches!(std_path(tcx, adt.did()).as_str(), "std::char::ToUppercase" | "std::char::ToLowercase"))
}

/// A `NonZero<T>` type (ADR 0177).
pub(crate) fn is_non_zero_ty(ty: Ty<'_>) -> bool {
    matches!(ty.kind(), ty::Adt(adt, _) if is_non_zero(adt.did()))
}

/// An operator that assigns, `a += b`: the operation it does.
pub(super) fn assign_operator(tcx: TyCtxt<'_>, id: DefId) -> Option<BinOp> {
    [
        (LangItem::AddAssign, BinOp::Add),
        (LangItem::SubAssign, BinOp::Sub),
        (LangItem::MulAssign, BinOp::Mul),
        (LangItem::DivAssign, BinOp::Div),
        (LangItem::RemAssign, BinOp::Rem),
        (LangItem::BitAndAssign, BinOp::BitAnd),
        (LangItem::BitOrAssign, BinOp::BitOr),
        (LangItem::BitXorAssign, BinOp::BitXor),
        (LangItem::ShlAssign, BinOp::Shl),
        (LangItem::ShrAssign, BinOp::Shr),
    ]
    .into_iter()
    .find_map(|(item, op)| tcx.is_lang_item(id, item).then_some(op))
}

/// An operator's trait, which the crate may implement: a value's, `a + b`,
/// an assignment's, `a += b`, each in its one list, and indexing (`Deref`
/// and `DerefMut` are `operational`, ADR 0394).
pub(super) fn is_operator(tcx: TyCtxt<'_>, id: DefId) -> bool {
    value_operator(tcx, id).is_some()
        || assign_operator(tcx, id).is_some()
        || [LangItem::Index, LangItem::IndexMut]
            .into_iter()
            .any(|item| tcx.is_lang_item(id, item))
}

impl<'a, 'tcx> Recognition<'a, 'tcx> {
    pub(super) fn bounded_by(&self, ty: ty::Ty<'tcx>, name: Symbol) -> bool {
        let ty = ty.peel_refs();
        let Some(trait_id) = self.tcx.get_diagnostic_item(name) else {
            return false;
        };
        // A type parameter, or an associated type of one, `<S as Source>::Iter`,
        // that only a caller knows (ADR 0106).
        let ty = self
            .tcx
            .try_normalize_erasing_regions(self.typing_env, ty::Unnormalized::new_wip(ty))
            .unwrap_or(ty);
        self.declared_bounds(ty, self.typing_env.param_env).contains(&trait_id)
    }

    /// The traits `ty`, a type only a caller knows, is bound by, their
    /// supertraits too: a type parameter's where-clauses', and an associated
    /// type's, and its trait's declaration's, `type Iter: Iterator` (ADR
    /// 0106). None of a type that's known.
    pub(super) fn declared_bounds(&self, ty: ty::Ty<'tcx>, param_env: ty::ParamEnv<'tcx>) -> Vec<DefId> {
        let declared = match ty.kind() {
            ty::Alias(
                _,
                ty::AliasTy {
                    kind: ty::Projection { def_id },
                    ..
                },
            ) => Some(self.tcx.item_bounds(*def_id).skip_binder()),
            _ => None,
        };
        let unknown = declared.is_some() || matches!(ty.kind(), ty::Param(_));
        let clauses = param_env.caller_bounds().iter().filter(|clause| {
            clause
                .as_trait_clause()
                .is_some_and(|bound| bound.self_ty().skip_binder() == ty)
        });
        let clauses = clauses.chain(declared.into_iter().flatten());
        clauses
            .filter(|_| unknown)
            .filter_map(|clause| clause.as_trait_clause().map(|bound| bound.def_id()))
            .collect()
    }

    /// A type only a caller knows, a type parameter or one's associated
    /// type, `H::Item: IntoIterator`: what it's given is an array or a JS
    /// iterator, which `Iterator.from` steps either of (ADR 0061). Not an
    /// associated type that's an iterator, `I::IntoIter`, a JS iterator
    /// already.
    pub(super) fn is_given_iterable(&self, ty: ty::Ty<'tcx>) -> bool {
        match ty.kind() {
            ty::Param(_) => true,
            _ => self.is_unknown(ty) && !self.is_generic_iter(ty),
        }
    }

    pub(super) fn is_generic_iter(&self, ty: ty::Ty<'tcx>) -> bool {
        self.bounded_by(ty, sym::Iterator)
    }

    /// A map's `VacantEntry`, `[m, key]` (ADR 0345), hashbrown's too (ADR 0423).
    pub(super) fn is_vacant_entry(&self, ty: ty::Ty<'tcx>) -> bool {
        matches!(ty.kind(), ty::Adt(adt, _) if map_entry_item(self.tcx, adt.did(), "VacantEntry"))
    }

    /// A map's `OccupiedEntry`, `[m, key]` (ADR 0345), hashbrown's too (ADR 0423).
    pub(super) fn is_occupied_entry(&self, ty: ty::Ty<'tcx>) -> bool {
        matches!(ty.kind(), ty::Adt(adt, _) if map_entry_item(self.tcx, adt.did(), "OccupiedEntry"))
    }
}

/// Is `id` std's, core's or alloc's?
/// Is `id` indexmap's `IndexMap` or `IndexSet`, std's maps that keep the
/// order they're added in (ADR 0424)?
fn is_index_map(tcx: TyCtxt<'_>, id: DefId) -> bool {
    tcx.crate_name(id.krate).as_str() == "indexmap" && matches!(bound_map_item(tcx, id), Some("HashMap" | "HashSet"))
}

/// std's map entry `name`, or that of a crate's map that's std's (ADRs
/// 0423, 0424).
fn map_entry_item(tcx: TyCtxt<'_>, id: DefId, name: &str) -> bool {
    match bound_map_item(tcx, id) {
        Some(item) => item == name,
        None => is_std_item(tcx, id) && tcx.item_name(id).as_str() == name,
    }
}

/// Which of std's map items an item of a crate whose maps are std's is:
/// hashbrown's, what std's are made of (ADR 0423), and indexmap's, kept in
/// the order they're added, as a JS `Map` is (ADR 0424). `HashMap`,
/// `HashSet`, or its map's `Entry`, `OccupiedEntry` or `VacantEntry`.
fn bound_map_item(tcx: TyCtxt<'_>, id: DefId) -> Option<&'static str> {
    match tcx.crate_name(id.krate).as_str() {
        // By its own path: its set's and its table's have entries too.
        "hashbrown" => Some(match ty::print::with_no_visible_paths!(tcx.def_path_str(id)).as_str() {
            "hashbrown::map::HashMap" => "HashMap",
            "hashbrown::set::HashSet" => "HashSet",
            "hashbrown::map::Entry" => "Entry",
            "hashbrown::map::OccupiedEntry" => "OccupiedEntry",
            "hashbrown::map::VacantEntry" => "VacantEntry",
            _ => return None,
        }),
        // By its name: its entries are its map's alone.
        "indexmap" => Some(match tcx.item_name(id).as_str() {
            "IndexMap" => "HashMap",
            "IndexSet" => "HashSet",
            "Entry" => "Entry",
            "OccupiedEntry" => "OccupiedEntry",
            "VacantEntry" => "VacantEntry",
            _ => return None,
        }),
        _ => None,
    }
}

pub(super) fn is_std_item(tcx: TyCtxt<'_>, id: DefId) -> bool {
    [sym::core, sym::alloc, sym::std].contains(&tcx.crate_name(id.krate))
}

pub(super) enum TraitCall {
    Clone,
    Default,
    Equality { negate: bool },
    Ordering,
}

impl<'a, 'tcx> Recognition<'a, 'tcx> {
    pub(super) fn trait_call(&self, method: DefId, trait_id: DefId) -> Option<TraitCall> {
        if self.tcx.is_lang_item(method, LangItem::CloneFn) {
            Some(TraitCall::Clone)
        } else if self.tcx.is_diagnostic_item(Symbol::intern("Default"), trait_id) {
            Some(TraitCall::Default)
        } else if self.tcx.is_lang_item(trait_id, LangItem::PartialEq) {
            Some(TraitCall::Equality {
                negate: self.tcx.item_name(method).as_str() == "ne",
            })
        } else if self.tcx.is_diagnostic_item(sym::Ord, trait_id)
            || self.tcx.is_lang_item(trait_id, LangItem::PartialOrd)
        {
            Some(TraitCall::Ordering)
        } else {
            None
        }
    }

    /// Resolve blanket Into/TryInto through From/TryFrom. The caller decides
    /// whether the resulting implementation is available for emission.
    /// A std method that only calls its bound's, `iter.sum::<S>()` that's
    /// `<S as Sum<Item>>::sum(iter)`: `product()`, `collect()` that's
    /// `FromIterator::from_iter`, and `parse()` that's `FromStr::from_str`.
    /// The bound's method, its arguments, and the impl's that it resolves to
    /// (ADR 0160). Its own type parameter, the iterator, is the receiver's.
    pub(super) fn resolve_delegated(
        &self,
        def_id: DefId,
        args: ty::GenericArgsRef<'tcx>,
    ) -> Option<(DefId, ty::GenericArgsRef<'tcx>, DefId)> {
        let tcx = self.tcx;
        if tcx.crate_name(def_id.krate) != sym::core {
            return None;
        }
        let wanted = match tcx.item_name(def_id).as_str() {
            "sum" => "sum",
            "product" => "product",
            "collect" => "from_iter",
            "parse" => "from_str",
            _ => return None,
        };
        tcx.clauses_of(def_id).clauses.iter().find_map(|&(clause, _)| {
            let bound = tcx
                .instantiate_bound_regions_with_erased(clause.as_trait_clause()?)
                .trait_ref;
            let method = tcx
                .associated_items(bound.def_id)
                .in_definition_order()
                .find(|item| item.is_fn() && item.name().as_str() == wanted)?
                .def_id;
            let bound_args = ty::EarlyBinder::bind(tcx, bound.args)
                .instantiate(tcx, args)
                .skip_normalization();
            let own = tcx.generics_of(method).own_params.len();
            let method_args = tcx.mk_args_from_iter(bound_args.iter().chain(args.types().take(own).map(Into::into)));
            let method_args = tcx
                .try_normalize_erasing_regions(self.typing_env, ty::Unnormalized::new_wip(method_args))
                .ok()?;
            let instance = super::items::resolve_instance_in(tcx, self.typing_env, method, method_args).ok()??;
            Some((method, method_args, instance.def_id()))
        })
    }

    pub(super) fn resolve_into(
        &self,
        def_id: DefId,
        args: ty::GenericArgsRef<'tcx>,
    ) -> Option<(DefId, ty::GenericArgsRef<'tcx>, DefId)> {
        let into = self.tcx.trait_of_assoc(def_id)?;
        let from = if self.tcx.is_diagnostic_item(sym::Into, into) {
            self.tcx.get_diagnostic_item(sym::From)?
        } else if self.tcx.is_diagnostic_item(sym::TryInto, into) {
            self.tcx.get_diagnostic_item(sym::TryFrom)?
        } else {
            return self.resolve_delegated(def_id, args);
        };
        let method = self
            .tcx
            .associated_items(from)
            .in_definition_order()
            .find(|item| item.is_fn())?
            .def_id;
        let args = self.tcx.mk_args(&[args[1], args[0]]);
        let args = self
            .tcx
            .try_normalize_erasing_regions(self.typing_env, ty::Unnormalized::new_wip(args))
            .ok()?;
        let instance = super::items::resolve_instance_in(self.tcx, self.typing_env, method, args).ok()??;
        Some((method, args, instance.def_id()))
    }
}

impl<'a, 'tcx> Recognition<'a, 'tcx> {
    pub(super) fn conversion(&self, adt: DefId, convert: StdItem, serialize: bool) -> Option<Ty<'tcx>> {
        struct Finder<'tcx> {
            tcx: TyCtxt<'tcx>,
            types: &'tcx ty::TypeckResults<'tcx>,
            convert: DefId,
            found: Option<Ty<'tcx>>,
        }
        impl<'tcx> intravisit::Visitor<'tcx> for Finder<'tcx> {
            fn visit_expr(&mut self, expr: &'tcx hir::Expr<'tcx>) {
                if let hir::ExprKind::Path(ref path) = expr.kind
                    && let hir::def::Res::Def(_, id) = self.types.qpath_res(path, expr.hir_id)
                    && self.tcx.trait_of_assoc(id) == Some(self.convert)
                {
                    self.found = Some(self.types.node_args(expr.hir_id).type_at(1));
                }
                intravisit::walk_expr(self, expr);
            }
        }
        let convert = self.tcx.get_diagnostic_item(convert.name())?;
        for owner in self.tcx.hir_body_owners() {
            let mut parent = self.tcx.opt_parent(owner.to_def_id());
            while let Some(id) = parent
                && serde_impl(self.tcx, id).is_none()
            {
                parent = self.tcx.opt_parent(id);
            }
            let Some(imp) = parent else { continue };
            let self_ty = self.tcx.type_of(imp).instantiate_identity().skip_normalization();
            if serde_impl(self.tcx, imp) != Some(serialize)
                || !matches!(self_ty.kind(), ty::Adt(a, _) if a.did() == adt)
            {
                continue;
            }
            let body = self.tcx.hir_body_owned_by(owner);
            let mut finder = Finder {
                tcx: self.tcx,
                types: self.tcx.typeck_body(body.id()),
                convert,
                found: None,
            };
            intravisit::Visitor::visit_body(&mut finder, body);
            if finder.found.is_some() {
                return finder.found;
            }
        }
        None
    }
}

impl<'a, 'tcx> Recognition<'a, 'tcx> {}

/// Is a struct's value its fields? The crate's own are, and std's
/// `PhantomData` and `Reverse(x)`, `[x]`; any other of std's has fields of
/// its own that aren't the JS value rust-js makes of it.
pub(super) fn struct_is_its_fields(tcx: TyCtxt<'_>, id: DefId) -> bool {
    ![sym::std, sym::core, sym::alloc].contains(&tcx.crate_name(id.krate))
        || tcx.is_lang_item(id, LangItem::PhantomData)
        || tcx.item_name(id).as_str() == "Reverse"
        // `Wrapping(x)`, `[x]` (ADR 0175).
        || tcx.is_diagnostic_item(Symbol::intern("Wrapping"), id)
}

pub(super) fn ordering_value(tcx: TyCtxt<'_>, enum_def: DefId, variant: Symbol) -> Option<i128> {
    if !tcx.is_lang_item(enum_def, LangItem::OrderingEnum) {
        return None;
    }
    Some(match variant.as_str() {
        "Less" => -1,
        "Equal" => 0,
        _ => 1,
    })
}

/// `dyn Iterator`, or `dyn DoubleEndedIterator` (ADR 0429), behind a
/// reference or a `Box`, or not.
pub(super) fn is_dyn_iter<'tcx>(tcx: TyCtxt<'tcx>, ty: Ty<'tcx>) -> bool {
    dyn_iter_trait(tcx, ty).is_some()
}

/// The trait of a `dyn Iterator` or a `dyn DoubleEndedIterator`, behind a
/// reference or a `Box`, or not.
pub(super) fn dyn_iter_trait<'tcx>(tcx: TyCtxt<'tcx>, ty: Ty<'tcx>) -> Option<DefId> {
    let ty = ty.peel_refs();
    let ty = ty.boxed_ty().unwrap_or(ty);
    let ty::Dynamic(traits, ..) = ty.kind() else {
        return None;
    };
    traits
        .principal_def_id()
        .filter(|&id| tcx.is_diagnostic_item(sym::Iterator, id) || is_double_ended(tcx, id))
}

/// Whether `ty` is a React context's `Provider` or `Consumer`, which
/// `react::provider` gives of a context, and React never changes.
pub(crate) fn is_context_member(tcx: TyCtxt<'_>, ty: Ty<'_>) -> bool {
    matches!(ty.kind(), ty::Adt(adt, _)
        if tcx.crate_name(adt.did().krate).as_str() == "react"
            && matches!(tcx.item_name(adt.did()).as_str(), "Provider" | "Consumer"))
}

/// What `size_of`, `align_of` and `type_name` tell of a type (ADR 0145).
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Debug)]
pub(crate) enum TypeFact {
    Size,
    Align,
    Name,
    /// Its `TypeId`'s key (ADR 0331).
    Id,
    /// `mem::needs_drop::<T>()`: whether dropping one runs code, as rustc
    /// says (ADR 0145).
    NeedsDrop,
    /// The class of a `HashMap` of it, and of a `HashSet`, keyed as its
    /// type says: `Map`, or `$KeyMap` of one found by value (ADR 0466).
    Map,
    Set,
    /// What std's unstable sort picks for its items by their layout, its
    /// small sort or partition (ADRs 0342, 0471).
    Sort,
}

impl TypeFact {
    /// Every fact: the one list of them.
    pub(crate) const ALL: [TypeFact; 8] = [
        TypeFact::Size,
        TypeFact::Align,
        TypeFact::Name,
        TypeFact::Id,
        TypeFact::NeedsDrop,
        TypeFact::Map,
        TypeFact::Set,
        TypeFact::Sort,
    ];

    /// Its word in the name of a type parameter's, `TSize`.
    /// The fact a word names, a library's manifest's (ADR 0145).
    pub(crate) fn of_word(word: &str) -> Option<TypeFact> {
        TypeFact::ALL.into_iter().find(|fact| fact.word() == word)
    }

    pub(crate) fn word(self) -> &'static str {
        match self {
            TypeFact::Size => "Size",
            TypeFact::Align => "Align",
            TypeFact::Name => "Name",
            TypeFact::Id => "Id",
            TypeFact::NeedsDrop => "NeedsDrop",
            TypeFact::Map => "Map",
            TypeFact::Set => "Set",
            TypeFact::Sort => "Sort",
        }
    }
}

/// A `HashMap` or a `HashSet` of a type parameter's keys, or a std
/// associated type's of one, `ty` itself: its key, and its class's fact,
/// `Map` or `Set`, which a caller gives (ADR 0466).
pub(crate) fn map_of_param<'tcx>(tcx: TyCtxt<'tcx>, ty: Ty<'tcx>) -> Option<(Ty<'tcx>, TypeFact)> {
    let ty::Adt(adt, args) = ty.kind() else { return None };
    let fact = match () {
        _ if is_std_def(tcx, adt.did(), StdItem::HashMap) => TypeFact::Map,
        _ if is_std_def(tcx, adt.did(), StdItem::HashSet) => TypeFact::Set,
        _ => return None,
    };
    let key = args.types().next()?;
    matches!(key.kind(), ty::Param(_) | ty::Alias(..)).then_some((key, fact))
}

/// A slice's unstable sort, or its `select_nth_unstable`, by its method's
/// name, of std's algorithm (ADR 0342).
fn unstable_sort(name: &str) -> Option<SliceOp> {
    let with = match name {
        "sort_unstable" | "select_nth_unstable" => SortWith::Order,
        "sort_unstable_by" | "select_nth_unstable_by" => SortWith::By,
        "sort_unstable_by_key" | "select_nth_unstable_by_key" => SortWith::Key,
        _ => return None,
    };
    Some(match name.starts_with("select") {
        true => SliceOp::SelectNth { with },
        false => SliceOp::SortUnstable { with },
    })
}

/// The items of a slice `def_id`, std's, sorts by std's unstable algorithm,
/// of its `args`, whose order its layout picks, which a caller of a generic
/// one gives, `TSort` (ADR 0471).
pub(crate) fn unstable_sort_items<'tcx>(
    tcx: TyCtxt<'tcx>,
    def_id: DefId,
    args: ty::GenericArgsRef<'tcx>,
) -> Option<Ty<'tcx>> {
    let imp = tcx.impl_of_assoc(def_id)?;
    (is_std_crate(tcx, def_id)
        && tcx.type_of(imp).skip_binder().is_slice()
        && unstable_sort(tcx.item_name(def_id).as_str()).is_some())
    .then(|| args.types().next())
    .flatten()
}

/// The layout `def_id` asks of its type argument, its size or alignment, and
/// whether of a value's: `size_of`, `align_of`, `size_of_val` and
/// `align_of_val`. None of any other function.
fn layout_query(tcx: TyCtxt<'_>, def_id: DefId) -> Option<(TypeFact, bool)> {
    let diagnostic = |name: &str| tcx.is_diagnostic_item(Symbol::intern(name), def_id);
    Some(match () {
        _ if diagnostic("mem_size_of") => (TypeFact::Size, false),
        _ if diagnostic("mem_size_of_val") => (TypeFact::Size, true),
        _ if diagnostic("mem_align_of") => (TypeFact::Align, false),
        // It has no diagnostic name.
        _ if std_path(tcx, def_id) == "std::mem::align_of_val" => (TypeFact::Align, true),
        _ => return None,
    })
}

/// The facts `def_id` asks of its type argument: `size_of` and
/// `size_of_val` its size, `align_of` its alignment, `Layout::new` both,
/// `type_name` and `type_name_of_val` its name. None of any other function.
pub(crate) fn type_facts(tcx: TyCtxt<'_>, def_id: DefId) -> &'static [TypeFact] {
    if let Some((fact, _)) = layout_query(tcx, def_id) {
        return match fact {
            TypeFact::Align => &[TypeFact::Align],
            _ => &[TypeFact::Size],
        };
    }
    match std_path(tcx, def_id).as_str() {
        "std::any::type_name" | "std::any::type_name_of_val" => return &[TypeFact::Name],
        "std::any::TypeId::of" => return &[TypeFact::Id],
        "std::mem::needs_drop" => return &[TypeFact::NeedsDrop],
        // Its size and its alignment (ADR 0417).
        "std::alloc::Layout::new" => return &[TypeFact::Size, TypeFact::Align],
        _ => {}
    }
    // `x.type_id()`, and a downcast's type, its last type argument (ADR 0331).
    let any = tcx.get_diagnostic_item(Symbol::intern("Any"));
    let name = tcx.item_name(def_id);
    if tcx.trait_of_assoc(def_id) == any && name.as_str() == "type_id" {
        return &[TypeFact::Id];
    }
    let Some(owner) = tcx.inherent_impl_of_assoc(def_id) else {
        return &[];
    };
    let owner = tcx.type_of(owner).instantiate_identity().skip_normalization();
    let of_any = owner.walk().any(|part| {
        part.as_type().is_some_and(|t| {
            matches!(t.kind(), ty::Dynamic(traits, ..)
            if traits.principal_def_id().is_some() && traits.principal_def_id() == any)
        })
    });
    match of_any && matches!(name.as_str(), "is" | "downcast_ref" | "downcast_mut" | "downcast") {
        true => &[TypeFact::Id],
        false => &[],
    }
}

/// Is `id` of a crate linked at two versions, whose paths alone don't tell
/// the two apart (ADR 0331)?
pub(crate) fn linked_twice(tcx: TyCtxt<'_>, id: DefId) -> bool {
    let name = tcx.crate_name(id.krate);
    tcx.crates(()).iter().filter(|&&c| tcx.crate_name(c) == name).count() > 1
}

/// `Any`, std's trait of a type's `TypeId` (ADR 0331).
pub(crate) fn any_trait(tcx: TyCtxt<'_>) -> Option<DefId> {
    tcx.get_diagnostic_item(Symbol::intern("Any"))
}

/// What a reference, a `Box` or an `Rc` points at, and any other type itself.
pub(crate) fn pointer_target<'tcx>(tcx: TyCtxt<'tcx>, ty: Ty<'tcx>) -> Ty<'tcx> {
    match ty.kind() {
        ty::Ref(_, inner, _) => *inner,
        ty::Adt(_, args) if ty.is_box() || rc_pointee(tcx, ty).is_some() => args.type_at(0),
        _ => ty,
    }
}

/// A `dyn Any`, `+ Send` or not, through a reference, a `Box` or an
/// `Rc` (ADR 0331).
pub(crate) fn points_at_dyn_any<'tcx>(tcx: TyCtxt<'tcx>, ty: Ty<'tcx>) -> bool {
    matches!(pointer_target(tcx, ty).kind(), ty::Dynamic(traits, ..) if traits.principal_def_id().is_some() && traits.principal_def_id() == any_trait(tcx))
}

/// What a `Copied` steps, its `I` of a `Copied<I>`, whose JS iterator it is,
/// each item a number or what's copied as it is (ADR 0480).
pub(crate) fn copied_source<'tcx>(tcx: TyCtxt<'tcx>, ty: Ty<'tcx>) -> Option<Ty<'tcx>> {
    match ty.kind() {
        ty::Adt(adt, args) if std_path(tcx, adt.did()) == "std::iter::Copied" => Some(args.type_at(0)),
        _ => None,
    }
}

/// What std's `Map` adapter of type `ty` maps, `I` of `Map<I, F>`, which a
/// map steps from either end where it knows its back (ADR 0455).
pub(crate) fn mapped_source<'tcx>(tcx: TyCtxt<'tcx>, ty: Ty<'tcx>) -> Option<Ty<'tcx>> {
    match ty.kind() {
        ty::Adt(adt, args) if std_path(tcx, adt.did()) == "std::iter::Map" => Some(args.type_at(0)),
        _ => None,
    }
}

/// How a function of type `ty`, in `owner`, is called: a closure's kind, or
/// a type parameter's or an associated type's, by the strongest `Fn` bound
/// its `owner` gives it, `F: FnMut`'s; `None` of what isn't one. One called
/// by `FnOnce` alone drops what it holds as its body ends, and one called by
/// `&` or `&mut` is dropped by what holds it (ADR 0470).
pub(crate) fn function_kind<'tcx>(tcx: TyCtxt<'tcx>, owner: DefId, ty: Ty<'tcx>) -> Option<ty::ClosureKind> {
    match ty.kind() {
        ty::Closure(_, args) => Some(args.as_closure().kind()),
        ty::Param(_) | ty::Alias(..) => (tcx.param_env(owner).caller_bounds().iter())
            .filter_map(|clause| clause.as_trait_clause())
            .filter(|bound| bound.self_ty().skip_binder() == ty)
            .filter_map(|bound| tcx.fn_trait_kind_from_def_id(bound.def_id()))
            .reduce(|a, b| if a.extends(b) { a } else { b }),
        _ => None,
    }
}

/// Whether the `Iterator` method `name`, an `IterComb`, leaves no item with
/// a destructor undropped (`IterComb::leaves_none`), or holds what it steps,
/// closed to drop it (`IterComb::holds_its_sources`).
pub(crate) fn leaves_none(name: &str) -> bool {
    methods::iterator(name).is_some_and(|comb| comb.leaves_none() || comb.holds_its_sources())
}

/// The type parameters whose names `{:?}` of `ty` shows, each a
/// `PhantomData<K>`'s, `PhantomData<{type_name::<K>()}>` (ADR 0452), in it
/// or a struct of the crate's it holds, through what points at it.
pub(crate) fn phantom_named<'tcx>(tcx: TyCtxt<'tcx>, ty: Ty<'tcx>) -> Vec<u32> {
    let mut found = Vec::new();
    phantom_named_in(tcx, pointer_target(tcx, ty), &mut Vec::new(), &mut found);
    found
}

/// `phantom_named` of a bound's type, where it's `Debug`'s: what `{:?}`
/// given that bound shows.
pub(crate) fn debug_named<'tcx>(tcx: TyCtxt<'tcx>, bound: ty::TraitRef<'tcx>) -> Vec<u32> {
    match tcx.is_diagnostic_item(Symbol::intern("Debug"), bound.def_id) {
        true => phantom_named(tcx, bound.self_ty()),
        false => Vec::new(),
    }
}

fn phantom_named_in<'tcx>(tcx: TyCtxt<'tcx>, ty: Ty<'tcx>, seen: &mut Vec<Ty<'tcx>>, found: &mut Vec<u32>) {
    let ty = ty.peel_refs();
    if seen.contains(&ty) {
        return;
    }
    seen.push(ty);
    match ty.kind() {
        ty::Adt(adt, args) if adt.is_phantom_data() => {
            if let ty::Param(param) = args.type_at(0).kind() {
                found.push(param.index);
            }
        }
        ty::Adt(adt, args) if adt.did().is_local() => {
            for field in adt.all_fields() {
                phantom_named_in(tcx, field.ty(tcx, args).skip_normalization(), seen, found);
            }
        }
        ty::Adt(_, args) => args.types().for_each(|t| phantom_named_in(tcx, t, seen, found)),
        ty::Tuple(items) => items.iter().for_each(|t| phantom_named_in(tcx, t, seen, found)),
        ty::Array(item, _) | ty::Slice(item) => phantom_named_in(tcx, *item, seen, found),
        _ => {}
    }
}

/// Does `ty` point at a `dyn Debug`, as a derived `Debug` gives each field?
pub(crate) fn points_at_dyn_debug<'tcx>(tcx: TyCtxt<'tcx>, ty: Ty<'tcx>) -> bool {
    matches!(pointer_target(tcx, ty).kind(), ty::Dynamic(traits, ..)
        if traits.principal_def_id().is_some_and(|id| tcx.is_diagnostic_item(Symbol::intern("Debug"), id)))
}

/// Does `owner` bound its type parameter `index` by `Any` (ADR 0331)?
pub(crate) fn bound_by_any(tcx: TyCtxt<'_>, owner: DefId, index: u32) -> bool {
    (tcx.clauses_of(owner).clauses.iter())
        .filter_map(|(clause, _)| clause.as_trait_clause())
        .map(|bound| bound.skip_binder())
        .any(|bound| {
            Some(bound.def_id()) == any_trait(tcx) && matches!(bound.self_ty().kind(), ty::Param(p) if p.index == index)
        })
}

/// Is `ty` a `fmt::Result` (ADR 0054)?
/// A write's result, `fmt::Result` or `io::Result<()>`: `undefined` where
/// it's `Ok` (ADRs 0054, 0132), its error thrown where it's an `Err` (ADRs
/// 0187, 0462), so `?` of one never breaks.
pub(crate) fn is_write_result_type<'tcx>(tcx: TyCtxt<'tcx>, ty: Ty<'tcx>) -> bool {
    is_fmt_result_type(tcx, ty) || is_io_unit_result_type(tcx, ty)
}

/// `io::Result<()>`, a write's result.
pub(crate) fn is_io_unit_result_type<'tcx>(tcx: TyCtxt<'tcx>, ty: Ty<'tcx>) -> bool {
    matches!(ty.kind(), ty::Adt(result, args) if is_std_def(tcx, result.did(), StdItem::Result)
        && args.type_at(0).is_unit()
        && matches!(args.type_at(1).kind(), ty::Adt(error, _) if std_path(tcx, error.did()) == "std::io::Error"))
}

pub(crate) fn is_fmt_result_type<'tcx>(tcx: TyCtxt<'tcx>, ty: Ty<'tcx>) -> bool {
    fmt_result(tcx).is_some_and(|result| tcx.erase_and_anonymize_regions(ty) == tcx.erase_and_anonymize_regions(result))
}

/// Is a result of `declared` type, a generic one, `Result<(), S::Error>`, a
/// `fmt::Result` once `args` are given, in `typing_env`? Generic code has it
/// as a `Result`, `{ TAG }`, where a `fmt::Result` is `undefined` or its error
/// thrown (ADRs 0187, 0461).
pub(crate) fn is_generic_fmt_result<'tcx>(
    tcx: TyCtxt<'tcx>,
    typing_env: ty::TypingEnv<'tcx>,
    declared: Ty<'tcx>,
    args: ty::GenericArgsRef<'tcx>,
) -> bool {
    // A `Result<(), E>` of a generic `E`: its `E` given `args`, a `fmt::Error`.
    let Some(ty::Adt(result, result_args)) = fmt_result(tcx).map(|result| *result.kind()) else {
        return false;
    };
    let ty::Adt(adt, declared_args) = *declared.kind() else {
        return false;
    };
    if adt != result
        || !declared_args.type_at(0).is_unit()
        || !ty::TypeVisitableExt::has_param(&declared_args.type_at(1))
    {
        return false;
    }
    let error = ty::EarlyBinder::bind(tcx, declared_args.type_at(1))
        .instantiate(tcx, args)
        .skip_normalization();
    let error = (tcx.try_normalize_erasing_regions(typing_env, ty::Unnormalized::new_wip(error))).unwrap_or(error);
    tcx.erase_and_anonymize_regions(error) == tcx.erase_and_anonymize_regions(result_args.type_at(1))
}

/// `fmt::Result`, what `Display::fmt` gives, if std's `Display` is loaded.
fn fmt_result(tcx: TyCtxt<'_>) -> Option<Ty<'_>> {
    let display = tcx.get_diagnostic_item(Symbol::intern("Display"))?;
    let fmt = tcx.associated_item_def_ids(display)[0];
    Some(tcx.fn_sig(fmt).skip_binder().skip_binder().output())
}

/// `cmp::max`, `Some(true)`, or `cmp::min`, `Some(false)`: `core`'s, which
/// `std` re-exports.
pub(crate) fn cmp_extreme(tcx: TyCtxt<'_>, def_id: DefId) -> Option<bool> {
    if tcx.crate_name(def_id.krate) != sym::core {
        return None;
    }
    match std_path(tcx, def_id).as_str() {
        "std::cmp::max" => Some(true),
        "std::cmp::min" => Some(false),
        _ => None,
    }
}

/// Is `variant` a `Result`'s `Err`?
pub(crate) fn is_err_variant(tcx: TyCtxt<'_>, variant: DefId) -> bool {
    tcx.is_lang_item(variant, LangItem::ResultErr)
}

/// The fmt trait whose `fmt` of its type argument `def_id` calls: a
/// placeholder's `Argument::new_display` its `Display`, `new_lower_hex` its
/// `LowerHex`, and `to_string` its `Display` (ADR 0187).
pub(crate) fn fmt_trait_called(tcx: TyCtxt<'_>, def_id: DefId) -> Option<DefId> {
    let named = |name: &str| match name {
        "Display" | "Debug" => tcx.get_diagnostic_item(Symbol::intern(name)),
        _ => tcx
            .all_traits_including_private()
            .find(|&id| is_other_fmt_trait(tcx, id) && tcx.item_name(id).as_str() == name),
    };
    if tcx.is_diagnostic_item(Symbol::intern("to_string_method"), def_id) {
        return named("Display");
    }
    let owner = tcx.inherent_impl_of_assoc(def_id)?;
    let ty::Adt(adt, _) = tcx.type_of(owner).instantiate_identity().skip_normalization().kind() else {
        return None;
    };
    if !tcx.is_lang_item(adt.did(), LangItem::FormatArgument) {
        return None;
    }
    named(match tcx.item_name(def_id).as_str() {
        "new_display" => "Display",
        "new_debug" => "Debug",
        "new_lower_hex" => "LowerHex",
        "new_upper_hex" => "UpperHex",
        "new_binary" => "Binary",
        "new_octal" => "Octal",
        "new_lower_exp" => "LowerExp",
        "new_upper_exp" => "UpperExp",
        "new_pointer" => "Pointer",
        _ => return None,
    })
}

/// What a writer asks its `Formatter` (ADRs 0137, 0143).
#[derive(Clone, Copy, PartialEq, Debug)]
pub(super) enum FormatterQuery {
    Alternate,
    Width,
    Precision,
    Fill,
    Align,
    SignPlus,
    SignAwareZeroPad,
}

/// What a `fmt::Result`'s method gives (ADR 0148), of an `Ok` and, where
/// it may be one, of an `Err` (ADR 0187).
#[derive(Clone, Copy)]
pub(super) enum FmtResultAnswer {
    /// `unwrap()`: `()`, or a panic.
    Unwrap,
    /// `expect(..)`: `()`, or a panic with its message.
    Expect,
    /// `is_ok()`, `is_err()`.
    Is(bool),
    /// `ok()`: `Some(())`, or `None` (ADR 0392).
    Ok,
    /// `map_err(f)`: `Ok(())`, or `Err(f(error))`, a `Result` of its own,
    /// arrayvec's `write_fmt(f).map_err(CapacityError::new)`.
    MapErr,
    /// `unwrap_or(())`, `unwrap_or_default()`: `()` either way.
    UnwrapOr,
    /// `map(f)`: `Ok(f(()))`, or the `Err`, writeable's
    /// `write_to(sink).map(Ok)`.
    Map,
}

/// The answer of the `fmt::Result` method `def_id`, if it's one of these.
pub(super) fn fmt_result_answer(tcx: TyCtxt<'_>, def_id: DefId) -> Option<FmtResultAnswer> {
    Some(match tcx.item_name(def_id).as_str() {
        "unwrap" => FmtResultAnswer::Unwrap,
        "expect" => FmtResultAnswer::Expect,
        "is_ok" => FmtResultAnswer::Is(true),
        "is_err" => FmtResultAnswer::Is(false),
        "ok" => FmtResultAnswer::Ok,
        "map_err" => FmtResultAnswer::MapErr,
        "map" => FmtResultAnswer::Map,
        "unwrap_or" | "unwrap_or_default" => FmtResultAnswer::UnwrapOr,
        _ => return None,
    })
}

/// A method of std's iterator traits that lowering from MIR writes as JS's
/// own (ADRs 0036, 0139, 0364): `iterator_method`'s answer.
#[derive(Clone, Copy, PartialEq)]
pub(crate) enum IterMethod {
    IntoIter,
    FromIter,
    Next,
    NextBack,
    /// `nth_back(n)`, `rfold(init, f)`, `rfind(p)`: stepped from the back
    /// (ADR 0455).
    NthBack,
    RFold,
    RFind,
    Len,
    SizeHint,
    Rev,
    Peekable,
    ByRef,
    Fuse,
    Map,
    Filter,
    SkipWhile,
    FlatMap,
    Take,
    Skip,
    ForEach,
    Any,
    All,
    Find,
    Position,
    /// `rposition(f)`, of a slice's items: from the back.
    RPosition,
    Eq,
    Ne,
    Enumerate,
    Fold,
    /// `reduce(f)`: `fold` from its first item, `None` of none (ADR 0479).
    Reduce,
    /// `try_fold(init, f)`, stopped at the first `None` or `Err` (ADR 0458).
    TryFold,
    Count,
    Sum,
    Product,
    Copied,
    Cloned,
    Max,
    Min,
    Last,
    Nth,
    Partition,
    Collect,
}

impl IterMethod {
    /// A lazy adapter that holds what it steps, so an owning source given it
    /// is `$closing`, dropping what it hasn't given as the adapter's closed
    /// (ADR 0465): `map`'s, `filter`'s, `take`'s and the rest.
    pub(crate) fn holds_its_source(self) -> bool {
        matches!(
            self,
            IterMethod::Map
                | IterMethod::Filter
                | IterMethod::SkipWhile
                | IterMethod::Take
                | IterMethod::Skip
                | IterMethod::Enumerate
                | IterMethod::Fuse
        )
    }
}

/// A `Debug` builder of std's, by its type, which a `Formatter`'s
/// `debug_struct(..)` and the like make (ADR 0136).
#[derive(Clone, Copy, PartialEq)]
pub(crate) enum DebugBuilder {
    Struct,
    Tuple,
    List,
    Set,
    Map,
}

/// A step of a `Debug` builder, by its method: what it's given, then what
/// it writes.
#[derive(Clone, Copy, PartialEq)]
pub(crate) enum BuilderStep {
    Field,
    Entry,
    Entries,
    Finish { non_exhaustive: bool },
}

/// The `Debug` builder `did`, a type, is: std's `DebugStruct` and the like.
pub(super) fn debug_builder(tcx: TyCtxt<'_>, did: DefId) -> Option<DebugBuilder> {
    if !std_path(tcx, did).starts_with("std::fmt::") {
        return None;
    }
    Some(match tcx.item_name(did).as_str() {
        "DebugStruct" => DebugBuilder::Struct,
        "DebugTuple" => DebugBuilder::Tuple,
        "DebugList" => DebugBuilder::List,
        "DebugSet" => DebugBuilder::Set,
        "DebugMap" => DebugBuilder::Map,
        _ => return None,
    })
}

/// The step of a `Debug` builder `def_id`, a method, is.
pub(super) fn builder_step(tcx: TyCtxt<'_>, def_id: DefId) -> Option<BuilderStep> {
    let owner = tcx
        .type_of(tcx.inherent_impl_of_assoc(def_id)?)
        .instantiate_identity()
        .skip_normalization();
    let ty::Adt(builder, _) = owner.kind() else {
        return None;
    };
    debug_builder(tcx, builder.did())?;
    Some(match tcx.item_name(def_id).as_str() {
        "field" => BuilderStep::Field,
        "entry" => BuilderStep::Entry,
        "entries" => BuilderStep::Entries,
        "finish" => BuilderStep::Finish { non_exhaustive: false },
        "finish_non_exhaustive" => BuilderStep::Finish { non_exhaustive: true },
        _ => return None,
    })
}

/// Whether `def_id`, a `Formatter`'s method, makes a `Debug` builder:
/// `debug_struct(..)` and the like, not what writes one whole,
/// `debug_struct_field1_finish`.
pub(super) fn makes_debug_builder(tcx: TyCtxt<'_>, def_id: DefId) -> bool {
    let name = tcx.item_name(def_id);
    name.as_str().starts_with("debug_") && !name.as_str().ends_with("_finish")
}

/// Whether `def_id` is a method of std's `Formatter`, its own.
fn is_formatter_method(tcx: TyCtxt<'_>, def_id: DefId) -> bool {
    tcx.inherent_impl_of_assoc(def_id).is_some_and(|imp| {
        matches!(tcx.type_of(imp).skip_binder().kind(),
            ty::Adt(adt, _) if is_std_def(tcx, adt.did(), StdItem::Formatter))
    })
}

/// The question `def_id` asks of a `Formatter`: `f.width()` and the like.
pub(super) fn formatter_query(tcx: TyCtxt<'_>, def_id: DefId) -> Option<FormatterQuery> {
    if !is_formatter_method(tcx, def_id) {
        return None;
    }
    Some(match tcx.item_name(def_id).as_str() {
        "alternate" => FormatterQuery::Alternate,
        "width" => FormatterQuery::Width,
        "precision" => FormatterQuery::Precision,
        "fill" => FormatterQuery::Fill,
        "align" => FormatterQuery::Align,
        "sign_plus" => FormatterQuery::SignPlus,
        "sign_aware_zero_pad" => FormatterQuery::SignAwareZeroPad,
        _ => return None,
    })
}

/// A std item another module asks about by name: only recognition.rs spells
/// std's names, as its diagnostic items, which the rest ask by this.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum StdItem {
    Any,
    Arc,
    ArrayIntoIter,
    AsMut,
    AsRef,
    Atomic,
    BinaryHeap,
    Borrow,
    BorrowMut,
    BoxNew,
    BTreeEntry,
    BTreeMap,
    BTreeSet,
    Cell,
    Debug,
    Default,
    Display,
    Eq,
    Error,
    FmtWrite,
    Formatter,
    IoWrite,
    From,
    Hash,
    HashMap,
    HashMapEntry,
    HashSet,
    FromIterator,
    Into,
    IntoIterator,
    Iterator,
    /// `LazyCell` or `LazyLock`, which have no diagnostic items, as `OnceCell`.
    LazyCell,
    LinkedList,
    LocalKey,
    MaybeUninit,
    Mutex,
    MutexGuard,
    /// `std::ptr::NonNull`, which is its pointer (ADR 0407).
    NonNull,
    /// `std::ops::RangeBounds`, a range's bounds, which generic code asks.
    RangeBounds,
    SliceIndex,
    /// `OnceCell` or `OnceLock`, which have no diagnostic items.
    OnceCell,
    Ord,
    PhantomData,
    Rc,
    RefCell,
    RefCellRef,
    RefCellRefMut,
    Result,
    RwLock,
    RwLockReadGuard,
    RwLockWriteGuard,
    SliceIter,
    ToString,
    TryFrom,
    Vec,
    VecDeque,
    Wrapping,
}

impl StdItem {
    /// Its diagnostic item's name in std.
    fn name(self) -> Symbol {
        match self {
            StdItem::Any => Symbol::intern("Any"),
            StdItem::Arc => sym::Arc,
            StdItem::ArrayIntoIter => Symbol::intern("ArrayIntoIter"),
            StdItem::AsMut => Symbol::intern("AsMut"),
            StdItem::AsRef => sym::AsRef,
            StdItem::Atomic => Symbol::intern("Atomic"),
            StdItem::BinaryHeap => Symbol::intern("BinaryHeap"),
            StdItem::Borrow => Symbol::intern("Borrow"),
            StdItem::BorrowMut => Symbol::intern("BorrowMut"),
            StdItem::BoxNew => Symbol::intern("box_new"),
            StdItem::BTreeEntry => Symbol::intern("BTreeEntry"),
            StdItem::BTreeMap => Symbol::intern("BTreeMap"),
            StdItem::BTreeSet => Symbol::intern("BTreeSet"),
            StdItem::Cell => Symbol::intern("Cell"),
            StdItem::Debug => Symbol::intern("Debug"),
            StdItem::Default => Symbol::intern("Default"),
            StdItem::Display => Symbol::intern("Display"),
            StdItem::Eq => sym::Eq,
            StdItem::Error => Symbol::intern("Error"),
            StdItem::FmtWrite => Symbol::intern("FmtWrite"),
            StdItem::Formatter => Symbol::intern("Formatter"),
            StdItem::IoWrite => Symbol::intern("IoWrite"),
            StdItem::From => sym::From,
            StdItem::Hash => sym::Hash,
            StdItem::HashMap => sym::HashMap,
            StdItem::HashMapEntry => Symbol::intern("HashMapEntry"),
            StdItem::HashSet => sym::HashSet,
            StdItem::FromIterator => sym::FromIterator,
            StdItem::Into => sym::Into,
            StdItem::IntoIterator => sym::IntoIterator,
            StdItem::Iterator => sym::Iterator,
            StdItem::LazyCell => Symbol::intern("LazyCell"),
            StdItem::LinkedList => Symbol::intern("LinkedList"),
            StdItem::LocalKey => Symbol::intern("LocalKey"),
            StdItem::MaybeUninit => Symbol::intern("MaybeUninit"),
            StdItem::Mutex => Symbol::intern("Mutex"),
            StdItem::MutexGuard => Symbol::intern("MutexGuard"),
            StdItem::NonNull => Symbol::intern("NonNull"),
            StdItem::RangeBounds => Symbol::intern("RangeBounds"),
            StdItem::SliceIndex => Symbol::intern("SliceIndex"),
            StdItem::OnceCell => Symbol::intern("OnceCell"),
            StdItem::Ord => sym::Ord,
            StdItem::PhantomData => Symbol::intern("PhantomData"),
            StdItem::Rc => sym::Rc,
            StdItem::RefCell => Symbol::intern("RefCell"),
            StdItem::RefCellRef => Symbol::intern("RefCellRef"),
            StdItem::RefCellRefMut => Symbol::intern("RefCellRefMut"),
            StdItem::Result => sym::Result,
            StdItem::RwLock => Symbol::intern("RwLock"),
            StdItem::RwLockReadGuard => Symbol::intern("RwLockReadGuard"),
            StdItem::RwLockWriteGuard => Symbol::intern("RwLockWriteGuard"),
            StdItem::SliceIter => Symbol::intern("SliceIter"),
            StdItem::ToString => Symbol::intern("ToString"),
            StdItem::TryFrom => sym::TryFrom,
            StdItem::Vec => sym::Vec,
            StdItem::VecDeque => Symbol::intern("VecDeque"),
            StdItem::Wrapping => Symbol::intern("Wrapping"),
        }
    }
}

/// Is `id` `mem::swap`, `mem::replace` or `mem::take`, which replace what a
/// `&mut` is to whole (ADR 0147)?
pub(crate) fn replaces_whole(tcx: TyCtxt<'_>, id: DefId) -> bool {
    tcx.is_diagnostic_item(Symbol::intern("mem_swap"), id)
        || tcx.is_diagnostic_item(Symbol::intern("mem_replace"), id)
        || tcx.crate_name(id.krate) == sym::core && std_path(tcx, id) == "std::mem::take"
}

/// Is `id` std's `item`?
/// What an `Rc` or `Arc` points at (ADR 0320).
pub(crate) fn rc_pointee<'tcx>(tcx: TyCtxt<'tcx>, ty: Ty<'tcx>) -> Option<Ty<'tcx>> {
    match ty.kind() {
        ty::Adt(adt, args)
            if tcx.is_diagnostic_item(sym::Rc, adt.did()) || tcx.is_diagnostic_item(sym::Arc, adt.did()) =>
        {
            Some(args.type_at(0))
        }
        _ => None,
    }
}

/// What an `rc::Weak` or `sync::Weak` points at (ADR 0320).
pub(crate) fn weak_pointee<'tcx>(tcx: TyCtxt<'tcx>, ty: Ty<'tcx>) -> Option<Ty<'tcx>> {
    match ty.kind() {
        ty::Adt(adt, args)
            if ["RcWeak", "ArcWeak"]
                .into_iter()
                .any(|name| tcx.is_diagnostic_item(Symbol::intern(name), adt.did())) =>
        {
            Some(args.type_at(0))
        }
        _ => None,
    }
}

/// Is `id` a function that reads an `Rc`'s or `Arc`'s counts, or makes
/// or reads a `Weak` (ADR 0320): what only a counted one can answer.
pub(crate) fn reads_rc_count(tcx: TyCtxt<'_>, id: DefId) -> bool {
    let Some(imp) = tcx.inherent_impl_of_assoc(id) else {
        return false;
    };
    let owner = tcx.type_of(imp).instantiate_identity().skip_normalization();
    weak_pointee(tcx, owner).is_some()
        || rc_pointee(tcx, owner).is_some()
            && matches!(
                tcx.item_name(id).as_str(),
                "strong_count"
                    | "weak_count"
                    | "get_mut"
                    | "make_mut"
                    | "try_unwrap"
                    | "into_inner"
                    | "unwrap_or_clone"
                    | "downgrade"
                    | "new_cyclic"
                    | "ptr_eq"
            )
}

/// Is `id` std's `FromStr`, which has no diagnostic item to know it by?
pub(crate) fn is_from_str(tcx: TyCtxt<'_>, id: DefId) -> bool {
    tcx.crate_name(id.krate) == sym::core
        && tcx.def_kind(id) == DefKind::Trait
        && std_path(tcx, id).ends_with("str::FromStr")
}

/// Is `id` a thread-local's `with_borrow` or `with_borrow_mut`, which runs
/// its closure while its `RefCell` is borrowed (ADR 0328)?
pub(super) fn is_local_borrow(tcx: TyCtxt<'_>, id: DefId) -> bool {
    local_key_access(tcx, id) == Some(false) && tcx.item_name(id).as_str() != "get"
}

/// Is `id` a thread-local's `get`, `set`, `with_borrow` or
/// `with_borrow_mut`, which only reads or sets what it holds (ADR 0270)?
/// `Some(true)` of `set`, which sets it.
pub(super) fn local_key_access(tcx: TyCtxt<'_>, id: DefId) -> Option<bool> {
    let imp = tcx.inherent_impl_of_assoc(id)?;
    let owner = tcx.type_of(imp).instantiate_identity().skip_normalization();
    if !matches!(owner.kind(), ty::Adt(adt, _) if is_std_def(tcx, adt.did(), StdItem::LocalKey)) {
        return None;
    }
    match tcx.item_name(id).as_str() {
        "set" => Some(true),
        "get" | "with_borrow" | "with_borrow_mut" => Some(false),
        _ => None,
    }
}

/// What a function asks of a `Vec`'s or a slice's length: its `len()`,
/// its `is_empty()`, or an item by its index, `v[i]` of a `Vec`. What
/// knows an index in bounds reads these (ADR 0292).
#[derive(Clone, Copy, PartialEq)]
pub(super) enum SliceLength {
    Len,
    IsEmpty,
    Index,
}

pub(super) fn slice_length<'tcx>(
    tcx: TyCtxt<'tcx>,
    def_id: DefId,
    args: ty::GenericArgsRef<'tcx>,
) -> Option<SliceLength> {
    // A `Vec` or a `VecDeque`, an array in JS either way.
    let vec = |ty: Ty<'tcx>| {
        matches!(ty.kind(), ty::Adt(adt, _)
            if tcx.is_diagnostic_item(sym::Vec, adt.did()) || is_std_def(tcx, adt.did(), StdItem::VecDeque))
    };
    if let Some(trait_) = tcx.trait_of_assoc(def_id) {
        let indexed = args.types().next().is_some_and(|ty| vec(ty.peel_refs()));
        return (tcx.is_lang_item(trait_, LangItem::Index)
            && indexed
            && args.types().nth(1).is_some_and(|i| i.is_usize()))
        .then_some(SliceLength::Index);
    }
    let owner = tcx
        .type_of(tcx.inherent_impl_of_assoc(def_id)?)
        .instantiate_identity()
        .skip_normalization();
    if !vec(owner) && !owner.is_slice() {
        return None;
    }
    match tcx.item_name(def_id).as_str() {
        "len" => Some(SliceLength::Len),
        "is_empty" => Some(SliceLength::IsEmpty),
        _ => None,
    }
}

pub(crate) fn is_std_def(tcx: TyCtxt<'_>, id: DefId, item: StdItem) -> bool {
    match item {
        // Known by its crate and name: `core`'s `OnceCell` and `LazyCell`,
        // `std`'s `OnceLock` and `LazyLock`.
        StdItem::OnceCell | StdItem::LazyCell => {
            let (cell, lock) = match item {
                StdItem::OnceCell => ("OnceCell", "OnceLock"),
                _ => ("LazyCell", "LazyLock"),
            };
            tcx.def_kind(id) == DefKind::Struct && {
                let name = tcx.item_name(id);
                name.as_str() == cell && tcx.crate_name(id.krate) == sym::core
                    || name.as_str() == lock && tcx.crate_name(id.krate) == sym::std
            }
        }
        // A lang item, which has no diagnostic item.
        StdItem::MaybeUninit => tcx.is_lang_item(id, LangItem::MaybeUninit),
        // A crate's whose maps are std's (ADRs 0423, 0424).
        StdItem::HashMap | StdItem::HashSet | StdItem::HashMapEntry if let Some(bound) = bound_map_item(tcx, id) => {
            let name = match item {
                StdItem::HashMap => "HashMap",
                StdItem::HashSet => "HashSet",
                _ => "Entry",
            };
            bound == name
        }
        _ => tcx.is_diagnostic_item(item.name(), id),
    }
}

/// A bound of a range, `RangeBounds`', of what the range is.
#[derive(Clone, Copy)]
pub(crate) enum RangeBound {
    /// Its field's value, `Included` or `Excluded` of it: `start` of `2..5`.
    Field(&'static str, &'static str),
    /// None, `Unbounded`: the end of `2..`.
    Unbounded,
    /// A pair of `Bound`s' item: `(Bound::Excluded(1), Bound::Unbounded)`'s.
    Item(usize),
}

/// The start and end bounds of a range of `ty`, a range of std's or a pair
/// of `Bound`s, as its `RangeBounds` gives them; `None` of anything else.
pub(crate) fn range_bounds(tcx: TyCtxt<'_>, ty: Ty<'_>) -> Option<(RangeBound, RangeBound)> {
    use RangeBound::{Field, Item, Unbounded};
    let lang = |item: LangItem| matches!(ty.kind(), ty::Adt(adt, _) if tcx.is_lang_item(adt.did(), item));
    Some(match () {
        _ if lang(LangItem::Range) => (Field("Included", "start"), Field("Excluded", "end")),
        _ if lang(LangItem::RangeFrom) => (Field("Included", "start"), Unbounded),
        _ if lang(LangItem::RangeTo) => (Unbounded, Field("Excluded", "end")),
        _ if lang(LangItem::RangeInclusiveStruct) => (Field("Included", "start"), Field("Included", "end")),
        _ if lang(LangItem::RangeToInclusive) => (Unbounded, Field("Included", "end")),
        _ if lang(LangItem::RangeFull) => (Unbounded, Unbounded),
        _ if matches!(ty.kind(), ty::Tuple(items) if items.len() == 2) => (Item(0), Item(1)),
        _ => return None,
    })
}

/// Std's `item`, which every std has: one of `core`'s, which a `#![no_std]`
/// crate has too. One of `alloc`'s or `std`'s may not be loaded.
pub(crate) fn std_item(tcx: TyCtxt<'_>, item: StdItem) -> DefId {
    tcx.get_diagnostic_item(item.name()).expect("std has it")
}

/// Std's `item` if the crate loads what has it: `alloc`'s `ToString` isn't
/// a `#![no_std]` crate's unless it loads `alloc`.
pub(crate) fn opt_std_item(tcx: TyCtxt<'_>, item: StdItem) -> Option<DefId> {
    tcx.get_diagnostic_item(item.name())
}

/// The method of `trait_id` named `name`, which it has.
pub(crate) fn trait_method(tcx: TyCtxt<'_>, trait_id: DefId, name: &str) -> DefId {
    tcx.associated_item_def_ids(trait_id)
        .iter()
        .copied()
        .find(|&id| tcx.item_name(id).as_str() == name)
        .expect("the trait has the method")
}

/// Is `id` the method `name` of std's trait `item`, `Error::source`?
pub(crate) fn is_std_method(tcx: TyCtxt<'_>, id: DefId, item: StdItem, name: &str) -> bool {
    tcx.item_name(id).as_str() == name && tcx.trait_of_assoc(id).is_some_and(|tr| is_std_def(tcx, tr, item))
}

/// `Clone::clone_from`, which a `Clone` dictionary has where its impl
/// writes its own (ADR 0324).
pub(crate) fn is_clone_from(tcx: TyCtxt<'_>, id: DefId) -> bool {
    tcx.item_name(id).as_str() == "clone_from"
        && tcx
            .trait_of_assoc(id)
            .is_some_and(|tr| tcx.is_lang_item(tr, LangItem::Clone))
}

/// A provided method of a std trait that rust-js's dictionary of it has,
/// which a `dyn` or generic code calls through it: `Error::source` (ADR
/// 0141), `fmt::Write`'s `write_char` and `write_fmt` (ADR 0180), and
/// `io::Write`'s `write_all` and `write_fmt` (ADR 0392).
pub(crate) fn in_std_dictionary(tcx: TyCtxt<'_>, id: DefId) -> bool {
    is_std_method(tcx, id, StdItem::Error, "source")
        || error_default(tcx, id).is_some()
        || is_writer_default(tcx, id)
        || is_std_method(tcx, id, StdItem::IoWrite, "write_all")
        || is_std_method(tcx, id, StdItem::IoWrite, "write_fmt")
        // A crate's `Hasher`'s provided methods, `$hasher`'s, and `Hash`'s
        // `hash_slice`, `$hashSlice`'s (ADR 0463).
        || tcx.trait_of_assoc(id).is_some_and(|tr| is_hasher(tcx, tr))
        || is_std_method(tcx, id, StdItem::Hash, "hash_slice")
}

/// `Error`'s deprecated `cause`, `Some(true)`, std's its `source`, or
/// `description`, `Some(false)`, std's a constant text, which generic code
/// calls through its dictionary (ADR 0141).
pub(crate) fn error_default(tcx: TyCtxt<'_>, id: DefId) -> Option<bool> {
    match () {
        _ if is_std_method(tcx, id, StdItem::Error, "cause") => Some(true),
        _ if is_std_method(tcx, id, StdItem::Error, "description") => Some(false),
        _ => None,
    }
}

/// `io::Write`'s `write_all`, `Some(true)`, or `write_fmt`, `Some(false)`,
/// which std's writes with `write` (ADR 0392).
pub(crate) fn io_write_default(tcx: TyCtxt<'_>, id: DefId) -> Option<bool> {
    match () {
        _ if is_std_method(tcx, id, StdItem::IoWrite, "write_all") => Some(true),
        _ if is_std_method(tcx, id, StdItem::IoWrite, "write_fmt") => Some(false),
        _ => None,
    }
}

/// `fmt::Write`'s `write_char` or `write_fmt`, which std's writes with
/// `write_str`: a `char`'s text, or a `write!`'s, which is a string in JS
/// (ADR 0166).
pub(crate) fn is_writer_default(tcx: TyCtxt<'_>, id: DefId) -> bool {
    is_std_method(tcx, id, StdItem::FmtWrite, "write_char") || is_std_method(tcx, id, StdItem::FmtWrite, "write_fmt")
}

/// `fmt::Arguments::new` and its kin, what `format_args!` makes.
pub(crate) fn is_arguments_new(tcx: TyCtxt<'_>, id: DefId) -> bool {
    tcx.item_name(id).as_str() == "new" && std_path(tcx, id).starts_with("std::fmt::Arguments")
}

/// `Argument::new_debug(&x)`, which `format_args!` makes of a `{:?}`
/// placeholder's argument (ADR 0137), as `classify` has it.
pub(crate) fn is_debug_argument(tcx: TyCtxt<'_>, id: DefId) -> bool {
    tcx.item_name(id).as_str() == "new_debug"
        && tcx.opt_parent(id).is_some_and(|parent| {
            matches!(tcx.def_kind(parent), DefKind::Impl { .. })
                && matches!(tcx.type_of(parent).skip_binder().kind(),
                    ty::Adt(adt, _) if tcx.is_lang_item(adt.did(), LangItem::FormatArgument))
        })
}

/// `f.pad(s)` of a `Formatter` (ADR 0143).
pub(crate) fn is_formatter_pad(tcx: TyCtxt<'_>, id: DefId) -> bool {
    tcx.item_name(id).as_str() == "pad" && is_formatter_method(tcx, id)
}

/// A std function that runs no code of the crate's (ADR 0069).
#[derive(Clone, Copy, PartialEq, Debug)]
pub(crate) enum PureStd {
    /// A question of what it's called on: `len()`, `is_some()`, `is_ok()`.
    Question,
    /// A copy or a string of it, which is pure where what it copies is:
    /// `clone()`, `to_owned()`, `to_string()`, `as_str()`.
    Copy,
    /// `Box::new(x)`, which is `x`.
    BoxNew,
    /// `unwrap_unchecked()`, which is its value, and can't panic.
    Unchecked,
}

/// Which pure std function `id` is, if it's one.
pub(crate) fn pure_std(tcx: TyCtxt<'_>, id: DefId) -> Option<PureStd> {
    if !is_std_item(tcx, id) {
        return None;
    }
    match tcx.item_name(id).as_str() {
        "len" | "is_empty" | "is_some" | "is_none" | "is_ok" | "is_err" if tcx.trait_of_assoc(id).is_none() => {
            Some(PureStd::Question)
        }
        "clone" | "to_owned" | "to_string" | "as_str" => Some(PureStd::Copy),
        "unwrap_unchecked" => Some(PureStd::Unchecked),
        _ if is_std_def(tcx, id, StdItem::BoxNew) => Some(PureStd::BoxNew),
        _ => None,
    }
}

/// A `Cell`'s or a `RefCell`'s method that reads, writes or lends what it
/// holds, of its name (ADRs 0025, 0074, 0153): `None` of another, `new` and
/// a `RefCell`'s borrows, which are their kinds'.
fn cell_method(name: &str, refcell: bool) -> Option<Std> {
    Some(match name {
        "get" | "set" | "update" | "from_mut" | "as_slice_of_cells" | "as_array_of_cells" if refcell => return None,
        "get" | "into_inner" => Std::CellGet,
        "set" => Std::CellSet,
        "update" => Std::CellUpdate,
        // A `&mut` as a `&Cell`, and a cell of each item.
        "from_mut" => Std::CellFromMut,
        "as_slice_of_cells" | "as_array_of_cells" => Std::CellItems,
        "get_mut" => Std::CellGetMut,
        "swap" => Std::CellSwap,
        "replace" => Std::CellReplace,
        "take" => Std::CellTake,
        "replace_with" if refcell => Std::CellReplaceWith,
        _ => return None,
    })
}
