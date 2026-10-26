//! Read-only library-operation recognition. No function lowering state or JS emission.

mod calls;
mod fmt;
mod inherent;
mod iterators;
mod json;
mod methods;
pub(super) mod registry;
mod serde;
mod specialization;
mod std_items;
mod traits;
mod types;
mod wrappers;

pub(super) use self::{
    calls::*, fmt::*, iterators::*, json::*, serde::*, specialization::*, std_items::*, traits::*, types::*,
    wrappers::*,
};

use super::combinators::{Comb, IterComb, IterSource, StepOp};

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
use super::std_types::number::{DurationOp, NumOp, SignMode};
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
use super::std_types::unwind::UnwindOp;
use rustc_ast::Mutability;
use rustc_hir::attrs::lang_items::LangItem;
use rustc_hir::def::DefKind;
use rustc_hir::{self as hir, intravisit};
use rustc_middle::mir::{BinOp, TerminatorKind, UnOp};

use rustc_middle::traits::ImplSource;
use rustc_middle::ty::{self, Ty, TyCtxt};
use rustc_span::def_id::{CRATE_MOD_ID, DefId, LocalDefId, LocalModId};
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
    /// `char::try_from(n)` of a `u32`, `to_char`, or `u8::try_from(c)` of a
    /// `char`, or their `try_into()`: `Ok` of it in range, or a
    /// `CharTryFromError` or a `TryFromCharError`, each its message.
    TryFromChar {
        to_char: bool,
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
    /// Unwinding: `catch_unwind` and `resume_unwind` (ADR 0660).
    Unwind(UnwindOp),
    /// The process a program runs in, Node's (ADR 0391): `env::args()`.
    Process(ProcessOp),
    /// memchr's searches, of the crate rustc builds (ADR 0529).
    Memchr(super::shipped::MemchrOp),
    /// `panic::Location`'s (ADR 0427).
    Location(super::std_types::location::LocationOp),
    /// A `Backtrace`'s (ADR 0568).
    Backtrace(super::std_types::backtrace::BacktraceOp),
    /// `std::future`'s: a future is a JS promise (ADR 0029).
    Future(super::std_types::future::FutureOp),
    /// A `fmt` trait's method called as a function on a `Formatter` held
    /// where it isn't its writer's, `Display::fmt(&x, self.0)` (ADR 0461).
    FmtTo(WriteCall),
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
    /// it's `N` long (ADR 0189); of a `Vec`, `owned`, moved, its `Err` the
    /// `Vec` given back (ADR 0557).
    SliceToArray {
        into: bool,
        owned: bool,
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
    /// `ManuallyDrop::drop(&mut m)`: the destructor of what `m` holds, in
    /// place, which `mem::drop(&mut m)`, of a reference, never runs (#171).
    DropPointee,
    /// `mem::swap(&mut a, &mut b)` and `mem::replace(&mut a, v)` of places:
    /// each written in turn.
    Swap,
    Replace,
    /// `dest.clone_from(&source)`: `mem::replace(dest, source.clone())`'s
    /// write, of a clone of the source.
    CloneFrom,
    /// `x.to_owned()` of a `T: Clone`, std's blanket impl, whose body is
    /// `self.clone()`: its clone.
    ToOwnedClone,
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
                // A `Peekable`'s `peek_mut`, `$peekCell`'s handle (ADR 0513).
                | Std::Step(StepOp::PeekMut)
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

impl<'a, 'tcx> Recognition<'a, 'tcx> {
    /// std's `FromStr`, found among the traits, as it has no diagnostic item.
    pub(super) fn std_from_str(&self) -> Option<DefId> {
        self.tcx
            .all_traits_including_private()
            .find(|&id| is_from_str(self.tcx, id))
    }
}
