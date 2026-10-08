//! Read-only library-operation recognition. No function lowering state or JS emission.

mod methods;

use super::combinators::{Comb, HeapOp, IterComb, IterSource, StepOp};
use super::format_spec::Radix;
use super::maps::{MapOp, Part};
use super::numbers::{DurationOp, NumOp};
use super::ranges::{RangeKind, RangeOp};
use super::representation::Num;
use super::text::{StringEdit, TextOp};
use rustc_ast::Mutability;
use rustc_hir::attrs::lang_items::LangItem;
use rustc_hir::def::DefKind;
use rustc_hir::{self as hir, intravisit};
use rustc_middle::mir::{BinOp, UnOp};
use rustc_middle::traits::ImplSource;
use rustc_middle::ty::{self, Ty, TyCtxt};
use rustc_span::def_id::{DefId, LocalDefId};
use rustc_span::hygiene::{ExpnKind, MacroKind};
use rustc_span::{Symbol, sym};

/// Only immutable analysis inputs: recognition cannot record dependencies,
/// allocate names, register helpers, or lower an expression.
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
    /// The argument itself: `Box::new(x)`, `Rc::new(x)`,
    /// `s.to_owned()`, `String::from(s)`, `v.iter()`, and `Deref` of
    /// `String`, `Rc`, `Vec`, `Ref`, `RefMut` and JS objects.
    Same,
    /// `o.as_ref()` and `o.as_mut()` of an `Option`: the `Option` its
    /// reference points at, whose items' references are the items (ADR 0023),
    /// and whose `&mut`s to what isn't an object a std call's items (ADR 0099).
    Pointee,
    /// `u64::from(x)` of a number that isn't a BigInt: `BigInt(x)` (ADR 0086).
    ToBig,
    /// `u8::try_from(x)` (`into: false`) or `x.try_into()` between
    /// integers: `Ok` of it in range, or a `TryFromIntError`.
    TryFromInt {
        into: bool,
    },
    /// An iterator's `cloned()` and `copied()`: its items, each cloned if
    /// that could be told apart from sharing it (ADR 0052).
    Cloned,
    /// An iterator's `fuse()`: the iterator, as an array and a JS iterator
    /// chain end at their first `None` and stay ended (ADR 0055).
    Fuse,
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
    /// `RefCell::borrow`, `borrow_mut`: the cell's `value`.
    Borrow,
    /// A `Mutex`'s `lock()` or an `RwLock`'s `read()` or `write()`: `Ok` of its `value`.
    Lock,
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
    /// `vec![a, b]`.
    VecMacro,
    Push,
    Len,
    /// `it.len()` of an `ExactSizeIterator`: how many items it has left.
    IterLen,
    /// `len()` of an iterator of the crate's whose `ExactSizeIterator` keeps
    /// std's `len`: its `size_hint()`, checked (ADR 0164).
    ExactLen,
    /// A `Duration`'s own, of its nanoseconds (ADR 0188).
    Duration(DurationOp),
    /// `panic!("{}", x)`: a panic of `x`'s `Display` (ADR 0189).
    PanicDisplay,
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
    /// `it.size_hint()` of a generic iterator: exact of an array, else
    /// `(0, None)` (ADR 0170).
    GenericSizeHint,
    /// An operator of `std::num::Wrapping`, its number's in its `[x]`
    /// (ADR 0175): `a * b`, `-a`, or, `true`, `a += b`.
    WrappingOp(Result<BinOp, UnOp>, bool),
    /// `write!(w, ..)` or `w.write_char(c)` of a writer of the crate's that
    /// keeps std's: its own `write_str`, given the text whole (ADR 0166).
    UserWrite,
    Clear,
    Retain,
    /// `panic!("..")`, `assert!(..)`: `throw new Error(..)`.
    Panic,
    /// `panic!("{}", x)`: the same, with a formatted message.
    PanicFmt,
    /// `panic!(x)` before edition 2021, `std::rt::begin_panic(x)`: the
    /// same, with `x` as it is, if it's text.
    BeginPanic,
    /// `size_of::<T>()`, `align_of::<T>()` and `size_of_val(&x)` of a
    /// sized `x`: the wasm32 target's, as their constants are (ADR 0090).
    SizeOf,
    AlignOf,
    SizeOfVal,
    /// `mem::drop(x)`: `x`'s destructor, and `mem::forget(x)`: none (ADR 0098).
    Drop,
    Forget,
    /// `mem::swap(&mut a, &mut b)` and `mem::replace(&mut a, v)` of places:
    /// each written in turn.
    Swap,
    Replace,
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
    /// `Option` (ADR 0030): `o != null`, `o == null`.
    IsSome,
    IsNone,
    /// `unwrap()` and `expect(msg)`: `$unwrap(o)`, `$unwrap(o, msg)`.
    Unwrap,
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
    /// `v.get(i)`: `v[i]`, which is `undefined` past the end.
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
    /// `collect()` into a `Result` or an `Option` of a collection: the first
    /// `Err` or `None`, or all the values.
    CollectFallible,
    /// `collect::<Vec<_>>()`: a new array, unless it's one already.
    Collect,
    Position,
    /// `max()` (true) or `min()` (false) of an iterator: an option.
    Extreme(bool),
    Chars,
    ToVec,
    Sort,
    SortBy,
    SortByKey,
    /// `a.cmp(&b)`: -1, 0 or 1 (ADR 0036).
    Cmp,
    /// `a.max(b)` (true) or `a.min(b)` (false) of two numbers.
    MaxOf(bool),
    /// An operator on references to numbers, `x % 10` with `x: &i32`,
    /// which rustc writes as a call of the operator's trait.
    Operator(BinOp),
    /// `-x` or `!b` of a reference to a number or a `bool`, likewise.
    UnaryOperator(UnOp),
    /// A thread-local's `with(f)`: `f(key)`; and `with_borrow(f)`,
    /// `with_borrow_mut(f)` of a `RefCell` one: `f(key.value)`.
    LocalWith,
    LocalBorrow,
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
}

impl Std {
    /// Does it take an iterator, and so a range as an array?
    pub(super) fn takes_iterator(self) -> bool {
        matches!(
            self,
            Std::ArrayMethod(_)
                | Std::Enumerate
                | Std::Rev
                | Std::Skip
                | Std::Take
                | Std::Fold
                | Std::Sum
                | Std::CollectString
                | Std::CollectFallible
                | Std::Collect
                | Std::Position
                | Std::Extreme(_)
                | Std::Last
                | Std::Cloned
                | Std::Fuse
                | Std::IterComb(_)
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
    /// Is `def_id` `bool::then` or `bool::then_some`, a child shown if a
    /// `bool` holds (ADR 0235)?
    pub(super) fn is_bool_then(&self, def_id: DefId) -> bool {
        matches!(self.tcx.item_name(def_id).as_str(), "then" | "then_some")
            && self.tcx.impl_of_assoc(def_id).is_some_and(|imp| {
                self.tcx
                    .type_of(imp)
                    .instantiate_identity()
                    .skip_normalization()
                    .is_bool()
            })
    }

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
        if diagnostic("mem_size_of") {
            return Some(Some(Std::SizeOf));
        }
        if diagnostic("mem_align_of") {
            return Some(Some(Std::AlignOf));
        }
        if diagnostic("mem_size_of_val") {
            return Some(Some(Std::SizeOfVal));
        }
        if diagnostic("mem_drop") {
            return Some(Some(Std::Drop));
        }
        if diagnostic("mem_forget") {
            return Some(Some(Std::Forget));
        }
        // Bytes to text, as Rust validates UTF-8 (ADR 0172).
        let utf8 = match std_path(tcx, def_id).as_str() {
            "std::str::from_utf8" | "std::str::<impl str>::from_utf8" => Some(TextOp::FromUtf8 { owned: false }),
            "std::string::String::from_utf8" => Some(TextOp::FromUtf8 { owned: true }),
            "std::str::from_utf8_unchecked" | "std::str::<impl str>::from_utf8_unchecked" => {
                Some(TextOp::Utf8Unchecked)
            }
            "std::string::String::from_utf8_lossy" => Some(TextOp::Utf8Lossy),
            "std::str::Utf8Error::valid_up_to" => Some(TextOp::Utf8Part("valid_up_to")),
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
            // A `Cow<str>`'s text: a JS string isn't changed in place, so it's
            // as owned as it'll be.
            "std::borrow::Cow::<'_, B>::into_owned" if args.types().next().is_some_and(|t| t.is_str()) => {
                Some(TextOp::Utf8Part("_0"))
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
        if diagnostic("mem_replace") {
            return Some(Some(Std::Replace));
        }
        if tcx.crate_name(def_id.krate) == sym::core && std_path(tcx, def_id) == "std::mem::take" {
            return Some(Some(Std::MemTake));
        }
        // `cmp::max(a, b)` of numbers is `a.max(b)`'s (ADR 0136); of anything
        // else, the call is lowered as `Ord::max`'s.
        if tcx.crate_name(def_id.krate) == sym::core
            && let Some(max) = match std_path(tcx, def_id).as_str() {
                "std::cmp::max" => Some(true),
                "std::cmp::min" => Some(false),
                _ => None,
            }
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
        if diagnostic("box_new") {
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
            return Some(Some(Std::PanicDisplay));
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
        if tcx.crate_name(def_id.krate) == sym::core && tcx.item_name(def_id).as_str() == "assert_failed" {
            return Some(Some(Std::AssertFailed));
        }
        if diagnostic("deref_method") || diagnostic("deref_mut_method") {
            // A reference to what's inside is the same JS value (ADR 0024 for JS objects).
            let Some(ty) = self_ty else { return Some(None) };
            // A `Cow<str>`'s text, borrowed or owned (ADR 0172).
            if diagnostic("deref_method") && self.is_cow_str(ty) {
                return Some(Some(Std::Text(TextOp::Utf8Part("_0"))));
            }
            let same = self.is_string_like(ty)
                || self.is_path_like(ty)
                || self.is_js_object(ty)
                || self.is_rc(ty)
                || self.is_vec_like(ty)
                || self.is_guard(ty);
            return Some(same.then_some(Std::Same));
        }
        None
    }

    /// A trait's method: an operator's, a comparison's, an iterator's, a
    /// conversion's.
    fn classify_trait_method(&self, def_id: DefId, trait_: DefId, args: ty::GenericArgsRef<'tcx>) -> Option<Std> {
        let tcx = self.tcx;
        let self_ty = args.types().next();
        let ty = self_ty?;
        // `x.borrow()` of std's `Borrow`: the value itself, as its dictionary's
        // is (ADR 0167). A type parameter's is its dictionary's.
        if is_std_def(tcx, trait_, StdItem::Borrow)
            && tcx.item_name(def_id).as_str() == "borrow"
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
        // `m[k]` of a map: its value, or a panic, as `get(k).expect(..)`.
        if tcx.is_lang_item(trait_, LangItem::Index) && self.is_map(ty) && !self.is_set(ty) {
            return Some(Std::Map(MapOp::Index));
        }
        // `&v[a..b]` of a slice, an array or a `Vec` (ADR 0063).
        if tcx.is_lang_item(trait_, LangItem::Index)
            && let Some(range) = args.types().nth(1)
            && self.range_kind(range).is_some()
            && (ty.peel_refs().is_slice() || ty.peel_refs().is_array() || self.is_vec_like(ty.peel_refs()))
        {
            return Some(Std::Text(TextOp::Slice));
        }
        // `&s[a..b]` of a string: by its UTF-8 bytes (ADR 0138).
        if tcx.is_lang_item(trait_, LangItem::Index)
            && let Some(range) = args.types().nth(1)
            && self.range_kind(range).is_some()
            && self.is_string_like(ty)
        {
            return Some(Std::Text(TextOp::StrSlice));
        }
        // `v[i]` of a `Vec` is a slice's, checked the same way.
        if (tcx.is_lang_item(trait_, LangItem::Index) || tcx.is_lang_item(trait_, LangItem::IndexMut))
            && self.is_vec_like(ty.peel_refs())
            && args.types().nth(1).is_some_and(|i| i.is_usize())
        {
            return Some(Std::Index);
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
        // `size_hint()` of an iterator of the crate's that keeps std's, or of
        // std's that knows its length (ADR 0170).
        if tcx.is_diagnostic_item(sym::Iterator, trait_) && tcx.item_name(def_id).as_str() == "size_hint" {
            if self.is_user_iterator(ty) {
                return Some(Std::SizeHint(false));
            }
            // A generic one is an array or a JS iterator (ADR 0061).
            if matches!(
                ty.peel_refs().kind(),
                ty::Param(_)
                    | ty::Alias(
                        _,
                        ty::AliasTy {
                            kind: ty::Projection { .. },
                            ..
                        }
                    )
            ) {
                return Some(Std::GenericSizeHint);
            }
            if self.range_kind(ty.peel_refs()).is_none()
                && self.is_array_iter(ty.peel_refs())
                && self.is_exact_size(ty.peel_refs())
            {
                return Some(Std::SizeHint(true));
            }
        }
        if tcx.is_diagnostic_item(sym::Iterator, trait_) {
            let collects_string = || {
                args.types()
                    .nth(1)
                    .is_some_and(|b| self.is_lang_adt(b, LangItem::String))
            };
            return Some(match tcx.item_name(def_id).as_str() {
                // A `Range`'s and an `a..`'s move its `start` (ADR 0129).
                "next"
                    if matches!(
                        self.range_kind(ty.peel_refs()),
                        Some(RangeKind::Exclusive | RangeKind::From)
                    ) =>
                {
                    Std::Range(RangeOp::Next)
                }
                // One of the crate's own is its impl's `next` (ADR 0055).
                "next" if !self.is_user_iterator(ty) => Std::Step(StepOp::Next),
                // The iterator itself, which knows where it is (ADR 0071), as
                // one of the crate's does.
                "by_ref" => Std::IterByRef,
                "peekable" => Std::Step(StepOp::Peekable),
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
                "fuse" => Std::Fuse,
                "collect" if collects_string() => Std::CollectString,
                "collect" if args.types().nth(1).is_some_and(|b| self.is_map(b)) => {
                    let set = args.types().nth(1).is_some_and(|b| self.is_set(b));
                    Std::Map(MapOp::From { set })
                }
                "collect"
                    if args
                        .types()
                        .nth(1)
                        .is_some_and(|b| self.is_std_type(b, StdItem::Result) || self.option_of(b).is_some()) =>
                {
                    Std::CollectFallible
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
        if std_path(tcx, trait_) == "std::iter::ExactSizeIterator"
            && tcx.item_name(def_id).as_str() == "len"
            && self.range_kind(ty.peel_refs()).is_none()
            && self.is_array_iter(ty.peel_refs())
        {
            return Some(Std::IterLen);
        }
        // Of a type parameter too: a generic one is an array or a JS iterator
        // (ADR 0061), never a collection of the crate's (ADR 0160). And of an
        // iterator of the crate's, std's for any iterator, the iterator itself
        // (ADR 0189).
        if tcx.is_diagnostic_item(sym::IntoIterator, trait_)
            && tcx.item_name(def_id).as_str() == "into_iter"
            && (ty.peel_refs().is_array()
                || ty.peel_refs().is_slice()
                || self.is_vec_like(ty.peel_refs())
                || matches!(ty.peel_refs().kind(), ty::Param(_))
                || self.is_user_iterator(ty))
        {
            return Some(Std::Same);
        }
        // 1.99's of a `Box<[T; N]>`, which is its array: through a `&mut`, a
        // handle on the box (ADR 0099).
        if tcx.is_diagnostic_item(sym::IntoIterator, trait_)
            && tcx.item_name(def_id).as_str() == "into_iter"
            && ty.peel_refs().boxed_ty().is_some_and(|inner| inner.is_array())
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
        // Writing to a standard stream (ADR 0132).
        if let Some(error) = self.stream(ty.peel_refs())
            && std_path(tcx, trait_) == "std::io::Write"
        {
            return match tcx.item_name(def_id).as_str() {
                "write_fmt" => Some(Std::Stream(StreamOp::Write { error })),
                "flush" => Some(Std::Stream(StreamOp::Nothing)),
                _ => None,
            };
        }
        // `v.extend(items)` (ADR 0062).
        if is_extend(tcx, trait_) && self.is_vec_like(ty.peel_refs()) {
            return Some(Std::Comb(Comb::Extend));
        }
        // `VecDeque::from(v)` is a copy of `v`, which may be a clone that
        // was never made (ADR 0052); `BinaryHeap::from(v)` puts one in heap order.
        if tcx.is_diagnostic_item(sym::From, trait_) && self.is_std_adt(ty, Symbol::intern("VecDeque")) {
            return Some(Std::ToVec);
        }
        if tcx.is_diagnostic_item(sym::From, trait_) && self.is_std_adt(ty, Symbol::intern("BinaryHeap")) {
            return Some(Std::Heap(HeapOp::From));
        }
        // `String::from_iter(items)` is `items.collect()` into a `String`.
        if tcx.is_diagnostic_item(sym::FromIterator, trait_) && self.is_lang_adt(ty, LangItem::String) {
            return Some(Std::CollectString);
        }
        // `HashMap::from([(k, v)])`: `new Map([[k, v]])`.
        if tcx.is_diagnostic_item(sym::From, trait_) && self.is_map(ty) {
            let set = self.is_set(ty);
            return Some(Std::Map(MapOp::From { set }));
        }
        // `[..].into()` of a map, a set, a queue, a heap or a `Vec`: what
        // its `from` is.
        if tcx.is_diagnostic_item(sym::Into, trait_)
            && let Some(to) = args.types().nth(1)
        {
            if self.is_map(to) {
                return Some(Std::Map(MapOp::From { set: self.is_set(to) }));
            }
            if self.is_std_adt(to, Symbol::intern("BinaryHeap")) {
                return Some(Std::Heap(HeapOp::From));
            }
            if self.is_std_adt(to, Symbol::intern("VecDeque")) || (self.is_std_adt(to, sym::Vec) && ty.is_array()) {
                return Some(Std::ToVec);
            }
        }
        // std's own conversions that change nothing in JS (ADR 0063): to a
        // `String` or a `Box<str>` (ADR 0191) from a `&str` or a `char`, and
        // between numbers, which only widen.
        let (from_ty, to_ty) = if tcx.is_diagnostic_item(sym::Into, trait_) {
            (Some(ty), args.types().nth(1))
        } else if tcx.is_diagnostic_item(sym::From, trait_) {
            (args.types().nth(1), Some(ty))
        } else {
            (None, None)
        };
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
                        if self.is_std_adt(from_ty, sym::Vec) && vec.type_at(0) == *item))
            {
                return Some(Std::Same);
            }
            // Into a BigInt from a number (ADR 0086), else the same.
            if let (Some(from), Some(to)) = (Num::of(from_ty.peel_refs()), Num::of(to_ty)) {
                return Some(if to.big() && !from.big() { Std::ToBig } else { Std::Same });
            }
        }
        // Between integers, which may not fit.
        let into = tcx.is_diagnostic_item(sym::TryInto, trait_);
        let (from_ty, to_ty) = if into {
            (Some(ty), args.types().nth(1))
        } else if tcx.is_diagnostic_item(sym::TryFrom, trait_) {
            (args.types().nth(1), Some(ty))
        } else {
            (None, None)
        };
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
        let to_owned = tcx.is_diagnostic_item(Symbol::intern("ToOwned"), trait_) && ty.is_str();
        (from_str || to_owned).then_some(Std::Same)
    }

    /// A method of a std type's own, by its type and its name.
    fn classify_inherent(&self, def_id: DefId, args: ty::GenericArgsRef<'tcx>) -> Option<Std> {
        let tcx = self.tcx;
        let self_ty = args.types().next();
        let owner = tcx
            .type_of(tcx.inherent_impl_of_assoc(def_id)?)
            .instantiate_identity()
            .skip_normalization();
        let adt = |name: &str| self.is_std_adt(owner, Symbol::intern(name));
        let string = self.is_lang_adt(owner, LangItem::String);
        let option = self.is_lang_adt(owner, LangItem::Option);
        let range = self.range_kind(owner);
        let result = self.is_std_adt(owner, sym::Result);
        let ordering = self.is_lang_adt(owner, LangItem::OrderingEnum);
        let local_key = adt("LocalKey");
        let arguments = self.is_lang_adt(owner, LangItem::FormatArguments);
        let argument = self.is_lang_adt(owner, LangItem::FormatArgument);
        let map = adt("HashMap") || adt("BTreeMap") || self.is_json_map(owner);
        let set = adt("HashSet") || adt("BTreeSet");
        let entry = adt("HashMapEntry") || adt("BTreeEntry");
        let name = tcx.item_name(def_id);
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
        let (deque, heap) = (adt("VecDeque"), adt("BinaryHeap"));
        // Theirs first: `push` and `pop` keep a heap's order, and a deque's
        // `remove` is an `Option` (ADR 0068).
        let peekable = self.is_peekable(owner);
        let chars = matches!(owner.kind(), ty::Adt(adt, _) if tcx.crate_name(adt.did().krate) == sym::core
            && tcx.item_name(adt.did()).as_str() == "Chars");
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
            "push" if heap => Some(Std::Heap(HeapOp::Push)),
            "pop" if heap => Some(Std::Heap(HeapOp::Pop)),
            "peek" if heap => Some(Std::First),
            "into_sorted_vec" if heap => Some(Std::Heap(HeapOp::IntoSorted)),
            "into_vec" if heap => Some(Std::Same),
            "remove" if deque => Some(Std::DequeRemove),
            "push_back" if deque => Some(Std::Push),
            "pop_back" if deque => Some(Std::Method("pop")),
            "push_front" if deque => Some(Std::Method("unshift")),
            "pop_front" if deque => Some(Std::Method("shift")),
            "front" if deque => Some(Std::First),
            "back" if deque => Some(Std::SliceLast),
            "make_contiguous" if deque => Some(Std::Same),
            // A `Vec` is its array, and the whole of it the same array: what's
            // written through `as_mut_slice()` is the `Vec`'s. An array's is it too.
            "as_slice" | "as_mut_slice" if adt("Vec") || owner.is_array() => Some(Std::Same),
            "drain" if adt("Vec") || deque => Some(Std::Text(TextOp::Drain)),
            "iter" | "iter_mut" if deque || heap => Some(Std::Same),
            "new" | "with_capacity" if deque || heap => Some(Std::VecNew),
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
            adt("Vec") || deque,
            adt("Vec") || deque || owner.is_slice(),
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
            "new" | "with_capacity" if map || set => Std::Map(MapOp::New { set }),
            "insert" if map => Std::Map(MapOp::Insert),
            "insert" if set => Std::Map(MapOp::Add),
            "get" | "get_mut" if map => Std::Map(MapOp::Get),
            "contains_key" if map => Std::Map(MapOp::Has),
            "contains" if set => Std::Map(MapOp::Has),
            "remove" if map => Std::Map(MapOp::Remove),
            "remove" if set => Std::Map(MapOp::Delete),
            "len" if map || set => Std::Map(MapOp::Len),
            "is_empty" if map || set => Std::Map(MapOp::IsEmpty),
            "iter" | "iter_mut" if map || set => Std::Map(MapOp::Iter(Part::Entries)),
            "keys" if map => Std::Map(MapOp::Iter(Part::Keys)),
            "values" | "values_mut" if map => Std::Map(MapOp::Iter(Part::Values)),
            "entry" if map => Std::Map(MapOp::Entry),
            "or_insert" if entry => Std::Map(MapOp::OrInsert),
            "or_insert_with" if entry => Std::Map(MapOp::OrInsertWith),
            "or_default" if entry => Std::Map(MapOp::OrDefault),
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
            "new" if adt("Rc") || adt("Arc") => Std::Same,
            // Kept for good, a `&'static` of it: in JS, which frees nothing
            // itself, the value itself.
            "leak" if adt("Vec") || string || owner.is_box() => Std::Same,
            "new" if adt("Cell") || adt("RefCell") || adt("Atomic") || adt("Mutex") || adt("RwLock") => Std::CellNew,
            // On one thread a lock is never contested: always `Ok` (ADR 0025).
            "lock" if adt("Mutex") => Std::Lock,
            "read" | "write" if adt("RwLock") => Std::Lock,
            "into_inner" | "get_mut" if adt("Mutex") || adt("RwLock") => Std::Lock,
            "get" if adt("Cell") => Std::CellGet,
            "set" if adt("Cell") => Std::CellSet,
            "replace" if adt("Cell") || adt("RefCell") => Std::CellReplace,
            "take" if adt("Cell") || adt("RefCell") => Std::CellTake,
            "replace_with" if adt("RefCell") => Std::CellReplaceWith,
            "borrow" | "borrow_mut" if adt("RefCell") => Std::Borrow,
            "load" | "into_inner" if adt("Atomic") => Std::AtomicLoad,
            "store" if adt("Atomic") => Std::AtomicStore,
            "swap" if adt("Atomic") => Std::AtomicSwap,
            "fetch_add" if adt("Atomic") => Std::AtomicFetch(BinOp::Add),
            "fetch_sub" if adt("Atomic") => Std::AtomicFetch(BinOp::Sub),
            "fetch_and" if adt("Atomic") => Std::AtomicFetch(BinOp::BitAnd),
            "fetch_or" if adt("Atomic") => Std::AtomicFetch(BinOp::BitOr),
            "fetch_xor" if adt("Atomic") => Std::AtomicFetch(BinOp::BitXor),
            "fetch_max" if adt("Atomic") => Std::AtomicFetchMax(true),
            "fetch_min" if adt("Atomic") => Std::AtomicFetchMax(false),
            "compare_exchange" | "compare_exchange_weak" if adt("Atomic") => Std::AtomicCompareExchange,
            "new" if adt("Vec") => Std::VecNew,
            "push" if adt("Vec") => Std::Push,
            // JS's `pop()` gives `undefined` when empty: `None` (ADR 0030).
            "pop" if adt("Vec") => Std::Method("pop"),
            "len" if adt("Vec") || owner.is_slice() => Std::Len,
            // A string counts its UTF-8 bytes, as Rust's does (ADR 0138).
            "len" if owner.is_str() || string => Std::Text(TextOp::ByteLen),
            "char_indices" if owner.is_str() => Std::Text(TextOp::CharIndices),
            "clear" if adt("Vec") => Std::Clear,
            "retain" if adt("Vec") => Std::Retain,
            "iter" | "iter_mut" if owner.is_slice() => Std::Same,
            "new" if string => Std::StringNew,
            "with_capacity" if string => Std::StringWithCapacity,
            "pop" if string => Std::StringEdit(StringEdit::Pop),
            "remove" if string => Std::StringEdit(StringEdit::Remove),
            "truncate" if string => Std::StringEdit(StringEdit::Truncate),
            "insert" | "insert_str" if string => Std::StringEdit(StringEdit::Insert),
            "retain" if string => Std::StringEdit(StringEdit::Retain),
            "clear" if string => Std::StringEdit(StringEdit::Clear),
            "as_str" if string => Std::Same,
            "trim" if owner.is_str() => Std::Trim { start: true, end: true },
            // A closure, a function or a set of `char`s as the pattern (ADRs 0063, 0157).
            "split" | "contains" if owner.is_str() && self_ty.is_some_and(|p| self.is_char_predicate(p)) => {
                Std::Text(if name.as_str() == "split" {
                    TextOp::SplitBy
                } else {
                    TextOp::ContainsBy
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
            "starts_with" | "ends_with" | "contains" | "replace" | "split" | "strip_prefix" | "strip_suffix"
            | "split_once" | "rsplit_once" | "find" | "rfind"
                if owner.is_str() && !self_ty.is_some_and(|p| self.is_string_like(p)) =>
            {
                return None;
            }
            // Splitting and searching by a `&str` or a `char` (ADR 0150); trimming by
            // a closure or a function too.
            "splitn" | "rsplitn" | "rsplit" | "split_terminator" | "match_indices" | "matches"
                if owner.is_str() && !self_ty.is_some_and(|p| self.is_string_like(p)) =>
            {
                return None;
            }
            "splitn" if owner.is_str() => Std::Text(TextOp::SplitN),
            "rsplitn" if owner.is_str() => Std::Text(TextOp::Rsplit(true)),
            "rsplit" if owner.is_str() => Std::Text(TextOp::Rsplit(false)),
            "split_terminator" if owner.is_str() => Std::Text(TextOp::SplitTerminator),
            "split_at" if owner.is_str() => Std::Text(TextOp::SplitAt),
            "match_indices" if owner.is_str() => Std::Text(TextOp::MatchIndices),
            "matches" if owner.is_str() => Std::Text(TextOp::Matches),
            "trim_matches" | "trim_start_matches" | "trim_end_matches"
                if owner.is_str() && !self_ty.is_some_and(|p| self.is_string_like(p) || self.is_char_predicate(p)) =>
            {
                return None;
            }
            "trim_matches" if owner.is_str() => Std::Text(TextOp::TrimMatches { start: true, end: true }),
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
            "split" if owner.is_str() => Std::Method("split"),
            "strip_prefix" if owner.is_str() => Std::StripPrefix,
            "strip_suffix" if owner.is_str() => Std::StripSuffix,
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
            "join" if owner.is_slice() => Std::Method("join"),
            "push_str" | "push" if string => Std::PushStr,
            "is_empty" if adt("Vec") || owner.is_slice() || owner.is_str() || string => Std::IsEmpty,
            "lock" if self.stream(owner).is_some() => Std::Stream(StreamOp::Nothing),
            "contains" if range.is_some() => Std::Range(RangeOp::Contains),
            "start" if range == Some(RangeKind::Inclusive) => Std::Range(RangeOp::Bound("start")),
            "end" if range == Some(RangeKind::Inclusive) => Std::Range(RangeOp::Bound("end")),
            "into_inner" if range == Some(RangeKind::Inclusive) => Std::Range(RangeOp::IntoInner),
            "is_empty" if matches!(range, Some(RangeKind::Exclusive | RangeKind::Inclusive)) => {
                Std::Range(RangeOp::IsEmpty)
            }
            "as_ref" | "as_mut" if option => Std::Pointee,
            // A `String`'s `&str` is the string, a `Vec`'s slice the array
            // (ADR 0211): of an `Option` of one, it's the option.
            "as_deref" | "as_deref_mut"
                if option
                    && self_ty
                        .is_some_and(|t| self.is_lang_adt(t, LangItem::String) || self.is_std_adt(t, sym::Vec)) =>
            {
                Std::Pointee
            }
            // A reference is the value (ADR 0023): what's in the `Result` is.
            "as_ref" if result => Std::Same,
            "into_inner" if adt("Cell") || adt("RefCell") => Std::CellGet,
            "to_ascii_lowercase" if owner.is_str() => Std::AsciiCase { upper: false },
            "to_ascii_uppercase" if owner.is_str() => Std::AsciiCase { upper: true },
            "eq_ignore_ascii_case" if owner.is_str() || owner.is_char() => Std::AsciiEq,
            "append" if adt("Vec") || adt("VecDeque") => Std::Append,
            "is_some" if option => Std::IsSome,
            "take" if option => Std::OptionTake,
            "replace" if option => Std::OptionReplace,
            "flatten" if option => Std::OptionFlatten,
            "iter" if option => Std::OptionIter,
            "copied" | "cloned" if option => Std::OptionCloned,
            "is_none" if option => Std::IsNone,
            "unwrap_or" if option => Std::UnwrapOr,
            "map" if option => Std::OptionMap,
            // A thread-local (ADR 0037) is its `Cell` or `RefCell`: `{ value }`.
            "with" if local_key => Std::LocalWith,
            "get" if local_key => Std::CellGet,
            "set" if local_key => Std::CellSet,
            "with_borrow" | "with_borrow_mut" if local_key => Std::LocalBorrow,
            "then" if ordering => Std::Then,
            "then_with" if ordering => Std::ThenWith,
            "reverse" if ordering => Std::Reverse,
            "chars" if owner.is_str() => Std::Chars,
            "to_vec" if owner.is_slice() => Std::ToVec,
            "sort" | "sort_unstable" if owner.is_slice() => Std::Sort,
            "sort_by" | "sort_unstable_by" if owner.is_slice() => Std::SortBy,
            "sort_by_key" | "sort_unstable_by_key" if owner.is_slice() => Std::SortByKey,
            "reverse" if owner.is_slice() => Std::Method("reverse"),
            // An array's `map` is JS's: a new array of what `f` makes of each.
            "map" if owner.is_array() => Std::Method("map"),
            // `v[0]` and `v.at(-1)` are `undefined` when `v` is empty: `None`.
            // A `_mut` one's item, of numbers or strings, is a handle on it
            // (ADR 0152).
            "first" | "first_mut" if owner.is_slice() => Std::First,
            "get" | "get_mut" if owner.is_slice() && args.types().nth(1).is_some_and(|i| i.is_usize()) => Std::SliceGet,
            "last" | "last_mut" if owner.is_slice() => Std::SliceLast,
            // `includes` compares strings and numbers by value, as `==` does,
            // but objects by identity: only for those.
            // A `&mut` to one is a cell, an object (ADR 0099): not by identity.
            "contains" if owner.is_slice() && self_ty.is_some_and(|t| self.compares_by_value(t)) => {
                Std::Method("includes")
            }
            "starts_with" | "ends_with" if owner.is_slice() && self_ty.is_some_and(|t| self.compares_by_value(t)) => {
                Std::Text(TextOp::SliceStartsWith {
                    end: name.as_str() == "ends_with",
                })
            }
            // Only `[u8]` has it.
            "eq_ignore_ascii_case" if owner.is_slice() => Std::Text(TextOp::BytesAsciiEq),
            "is_ok" if result => Std::IsOk(true),
            "is_err" if result => Std::IsOk(false),
            "ok" if result => Std::ResultOk,
            // A write's `io::Result<()>` is always `Ok`, and nothing (ADR 0132).
            "unwrap" | "expect"
                if result
                    && args.types().next().is_some_and(|t| t.is_unit())
                    && args.types().nth(1).is_some_and(|e| self.is_io_error(e)) =>
            {
                Std::Stream(StreamOp::Nothing)
            }
            "unwrap" | "expect" if result => Std::UnwrapOk,
            "unwrap_err" | "expect_err" if result => Std::UnwrapErr,
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

    /// `io::Result<()>`, which a write gives: always `Ok`, as nothing rust-js
    /// writes to fails (ADR 0132), and so nothing, as a `fmt::Result` is.
    pub(super) fn is_io_unit_result(&self, ty: Ty<'tcx>) -> bool {
        matches!(ty.kind(), ty::Adt(_, args) if self.is_std_adt(ty, sym::Result)
            && args.type_at(0).is_unit()
            && self.is_io_error(args.type_at(1)))
    }

    fn is_io_error(&self, ty: Ty<'tcx>) -> bool {
        matches!(ty.kind(), ty::Adt(error, _) if std_path(self.tcx, error.did()) == "std::io::Error")
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
        self.is_std_adt(ty, item.name())
    }

    pub(super) fn is_std_adt(&self, ty: Ty<'tcx>, name: Symbol) -> bool {
        matches!(ty.kind(), ty::Adt(adt, _) if self.tcx.is_diagnostic_item(name, adt.did()))
    }

    pub(super) fn is_lang_adt(&self, ty: Ty<'tcx>, item: LangItem) -> bool {
        matches!(ty.kind(), ty::Adt(adt, _) if self.tcx.is_lang_item(adt.did(), item))
    }

    pub(super) fn is_vec_like(&self, ty: Ty<'tcx>) -> bool {
        self.is_std_adt(ty, sym::Vec)
            || ["VecDeque", "BinaryHeap"]
                .into_iter()
                .any(|name| self.is_std_adt(ty, Symbol::intern(name)))
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
        ["HashMap", "HashSet", "BTreeMap", "BTreeSet"]
            .into_iter()
            .any(|name| self.is_std_adt(ty, Symbol::intern(name)))
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
        self.is_std_adt(ty, Symbol::intern("HashSet")) || self.is_std_adt(ty, Symbol::intern("BTreeSet"))
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

    /// A `dyn Iterator`, boxed or lent: a JS iterator, whatever made it.
    pub(super) fn is_dyn_iter(&self, ty: Ty<'tcx>) -> bool {
        is_dyn_iter(self.tcx, ty)
    }

    /// core's `DoubleEndedIterator`, found among the traits, as it has no
    /// diagnostic item.
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

    /// Does JS's `===` compare a `ty` as `==` does: a number, a string or a
    /// `bool`. A `&mut` to one is a cell, an object (ADR 0099): not by value.
    fn compares_by_value(&self, ty: Ty<'tcx>) -> bool {
        !ty.walk()
            .any(|part| matches!(part.as_type().map(|p| *p.kind()), Some(ty::Ref(_, _, Mutability::Mut))))
            && (self.is_string_like(ty) || Num::of(ty).is_some() || ty.is_bool())
    }

    /// Is `ty` an `ExactSizeIterator`, whose `size_hint()` std makes exact?
    fn is_exact_size(&self, ty: Ty<'tcx>) -> bool {
        let Some(exact) = self
            .tcx
            .all_traits_including_private()
            .find(|&id| is_iterator_extension(self.tcx, id) && self.tcx.item_name(id).as_str() == "ExactSizeIterator")
        else {
            return false;
        };
        let tr = ty::TraitRef::new(self.tcx, exact, [self.tcx.erase_and_anonymize_regions(ty)]);
        self.tcx
            .codegen_select_candidate(self.typing_env.as_query_input(tr))
            .is_ok()
    }

    pub(super) fn double_ended_iterator(&self) -> Option<DefId> {
        self.tcx
            .all_traits_including_private()
            .find(|&id| is_iterator_extension(self.tcx, id) && self.tcx.item_name(id).as_str() == "DoubleEndedIterator")
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
}

impl<'a, 'tcx> Recognition<'a, 'tcx> {
    pub(super) fn json_call(&self, def_id: DefId, generic_args: ty::GenericArgsRef<'tcx>) -> Option<JsonCall<'tcx>> {
        let tcx = self.tcx;
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
            && self.is_std_adt(from, sym::Vec)
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
                .is_some_and(|this| self.is_std_adt(this, Symbol::intern("Formatter")))
    }

    /// A `Formatter`, whose text is a string in JS (ADR 0054).
    pub(super) fn is_formatter_type(&self, ty: Ty<'tcx>) -> bool {
        self.is_std_adt(ty, Symbol::intern("Formatter"))
    }

    fn is_formatter(&self, ty: Ty<'tcx>) -> bool {
        matches!(ty.kind(), ty::Ref(_, inner, Mutability::Mut)
            if self.is_std_adt(*inner, Symbol::intern("Formatter")))
    }
    pub(super) fn display_trait(&self) -> DefId {
        self.tcx
            .get_diagnostic_item(Symbol::intern("Display"))
            .expect("std has `Display`")
    }
    pub(super) fn is_fmt_result(&self, ty: Ty<'tcx>) -> bool {
        let fmt = self.tcx.associated_item_def_ids(self.display_trait())[0];
        let result = self.tcx.fn_sig(fmt).skip_binder().skip_binder().output();
        self.tcx.erase_and_anonymize_regions(ty) == self.tcx.erase_and_anonymize_regions(result)
    }
    /// Is `ty` a `fmt::Error`, a `fmt::Result`'s `Err` (ADR 0187)?
    pub(super) fn is_fmt_error(&self, ty: Ty<'tcx>) -> bool {
        let fmt = self.tcx.associated_item_def_ids(self.display_trait())[0];
        let result = self.tcx.fn_sig(fmt).skip_binder().skip_binder().output();
        matches!(result.kind(), ty::Adt(_, args) if self.tcx.erase_and_anonymize_regions(ty) == self.tcx.erase_and_anonymize_regions(args.type_at(1)))
    }
    pub(super) fn formatter_param(&self, def_id: DefId) -> Option<usize> {
        let sig = self.tcx.fn_sig(def_id).skip_binder().skip_binder();
        if !self.is_fmt_result(sig.output()) {
            return None;
        }
        sig.inputs().iter().position(|&t| self.is_formatter(t))
    }

    pub(super) fn write_call(&self, def_id: DefId, known_function: bool) -> WriteCall {
        let tcx = self.tcx;
        let trait_id = tcx.trait_of_assoc(def_id);
        let is_trait = |name: &str| trait_id.is_some_and(|t| tcx.is_diagnostic_item(Symbol::intern(name), t));
        let owner = tcx
            .inherent_impl_of_assoc(def_id)
            .map(|imp| tcx.type_of(imp).instantiate_identity().skip_normalization());
        let on_formatter = owner.is_some_and(|t| self.is_std_adt(t, Symbol::intern("Formatter")));
        match tcx.item_name(def_id).as_str() {
            "write_fmt" | "write_str" | "write_char" if on_formatter || is_trait("FmtWrite") => WriteCall::Text,
            "pad" if on_formatter => WriteCall::Pad,
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
            && ["ParseIntError", "ParseFloatError", "ParseBoolError", "ParseCharError", "TryFromIntError"]
                .contains(&self.tcx.item_name(adt.did()).as_str()))
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
            ty::Adt(adt, args) if self.tcx.is_diagnostic_item(Symbol::intern("Wrapping"), adt.did()) => {
                Some(args.type_at(0))
            }
            _ => None,
        }
    }

    /// A `Cow<str>`, `{ TAG, _0 }`: its text, borrowed or owned (ADR 0172).
    pub(super) fn is_cow_str(&self, ty: Ty<'tcx>) -> bool {
        matches!(ty.kind(), ty::Adt(adt, args) if std_path(self.tcx, adt.did()) == "std::borrow::Cow"
            && args.types().next().is_some_and(|t| t.is_str()))
    }

    pub(super) fn is_json_error(&self, ty: Ty<'tcx>) -> bool {
        matches!(ty.kind(), ty::Adt(adt, _) if self.tcx.crate_name(adt.did().krate).as_str() == "serde_json"
            && self.tcx.item_name(adt.did()).as_str() == "Error")
    }

    pub(super) fn is_reverse(&self, ty: Ty<'tcx>) -> bool {
        matches!(ty.kind(), ty::Adt(adt, _) if self.tcx.crate_name(adt.did().krate) == rustc_span::sym::core
            && self.tcx.item_name(adt.did()).as_str() == "Reverse")
    }

    /// std's iterator over an array's items, whose JS value is the array
    /// (ADR 0061): `Some(false)` of `slice::Iter`, which borrows them, and
    /// `Some(true)` of `array::IntoIter` and `vec::IntoIter`, which own them.
    pub(super) fn array_source(&self, ty: Ty<'tcx>) -> Option<bool> {
        let ty = self.reveal(ty);
        let ty::Adt(adt, _) = ty.kind() else { return None };
        if self.is_std_type(ty, StdItem::SliceIter) {
            return Some(false);
        }
        let vec_into_iter =
            self.tcx.crate_name(adt.did().krate) == sym::alloc && std_path(self.tcx, adt.did()) == "std::vec::IntoIter";
        (self.is_std_type(ty, StdItem::ArrayIntoIter) || vec_into_iter).then_some(true)
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
                    "std::str::Bytes",
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
                    "std::char::ToUppercase",
                    "std::char::ToLowercase",
                    "std::collections::vec_deque::Iter",
                    "std::collections::vec_deque::IntoIter",
                    "std::collections::binary_heap::Iter",
                    "std::collections::binary_heap::IntoIter",
                    "std::option::Iter",
                    "std::option::IntoIter",
                ]
                .contains(&path.as_str())
                || self.is_str_split(ty))
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
                .try_normalize_erasing_regions(self.typing_env, ty::Unnormalized::new_wip(ty))
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

    pub(super) fn reveal(&self, ty: Ty<'tcx>) -> Ty<'tcx> {
        if !rustc_middle::ty::TypeVisitableExt::has_opaque_types(&ty) {
            return ty;
        }
        self.tcx
            .try_normalize_erasing_regions(self.typing_env, ty::Unnormalized::new_wip(ty))
            .unwrap_or(ty)
    }

    pub(super) fn is_std_wrapper(&self, ty: Ty<'tcx>) -> bool {
        ty.is_box()
            || self.is_lang_adt(ty, LangItem::String)
            || self.is_rc(ty)
            || self.is_guard(ty)
            || ["Cell", "RefCell", "Atomic", "Mutex", "RwLock"]
                .into_iter()
                .any(|name| self.is_std_adt(ty, Symbol::intern(name)))
            || self.is_vec_like(ty)
    }

    /// An `Rc`, or an `Arc`, which on one thread is one: the value it
    /// points at, shared (ADR 0023).
    pub(super) fn is_rc(&self, ty: Ty<'tcx>) -> bool {
        self.is_std_adt(ty, sym::Rc) || self.is_std_adt(ty, sym::Arc)
    }

    /// Is std's `Borrow<to>` of a `from` the same JS value (ADR 0167): a
    /// value as itself, through a reference, a `Box` or an `Rc` (ADR 0023),
    /// text as a `str`, and a `Vec` or an array as a slice.
    pub(super) fn borrows_as_itself(&self, from: Ty<'tcx>, to: Ty<'tcx>) -> bool {
        let pointee = |mut ty: Ty<'tcx>| loop {
            ty = match ty.kind() {
                ty::Ref(_, inner, _) => *inner,
                ty::Adt(_, args) if ty.is_box() || self.is_rc(ty) => args.type_at(0),
                _ => break ty,
            };
        };
        let (from, to) = (pointee(from), pointee(to));
        from == to
            || (self.is_string_like(from) && to.is_str())
            || ((from.is_array() || self.is_std_adt(from, sym::Vec)) && to.is_slice())
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
            "RefCellRef",
            "RefCellRefMut",
            "MutexGuard",
            "RwLockReadGuard",
            "RwLockWriteGuard",
        ]
        .into_iter()
        .any(|name| self.is_std_adt(ty, Symbol::intern(name)))
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

/// Is `id` core's `Sum` or `Product`, of `sum()` and `product()`, which
/// have no diagnostic items?
pub(crate) fn is_sum_or_product(tcx: TyCtxt<'_>, id: DefId) -> bool {
    tcx.crate_name(id.krate) == sym::core
        && tcx.def_kind(id) == DefKind::Trait
        && [Symbol::intern("Sum"), Symbol::intern("Product")].contains(&tcx.item_name(id))
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
    pub(super) fn catching(&self, id: DefId) -> Catching {
        let output = self.tcx.fn_sig(id).skip_binder().skip_binder().output();
        if self.is_std_adt(output, sym::Result) {
            Catching::Result
        } else if matches!(output.kind(), ty::Adt(adt, args) if self.is_js_object(output)
            && self.tcx.item_name(adt.did()).as_str() == "Promise"
            && args.types().next().is_some_and(|t| self.is_std_adt(t, sym::Result)))
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
        // Its evidence is the writer or reader itself (ADR 0081).
        || serde_trait(tcx, id).is_some()
        // `parse` of a `T: FromStr`: its dictionary's `from_str` (ADR 0161).
        || is_from_str(tcx, id)
        // `s.as_ref()` of an `S: AsRef<str>`: its dictionary's `as_ref` (ADR 0162).
        || is_std_def(tcx, id, StdItem::AsRef)
        // `key.borrow()` of a `K: Borrow<Q>`, as a map's lookup is: its
        // dictionary's `borrow` (ADR 0167).
        || is_std_def(tcx, id, StdItem::Borrow)
        // `{:x}`, `{:e}` and `{:p}` of the crate's types (ADR 0165), and of a
        // `T: LowerHex` or `T: Pointer` (ADR 0174).
        || is_other_fmt_trait(tcx, id)
        // `a + b` of a `T: Add`, and `x.into()` of a `T: Into<U>` (ADR 0108).
        || value_operator(tcx, id).is_some()
        || tcx.is_diagnostic_item(sym::Into, id)
        // A writer of the crate's own, given its text a `write!` at a time
        // (ADR 0166), and generic code that writes to any writer, a `W:
        // fmt::Write`: its dictionary's `write_str`, `write_char` and
        // `write_fmt` (ADR 0180).
        || tcx.is_diagnostic_item(Symbol::intern("FmtWrite"), id)
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
        || tcx.is_diagnostic_item(sym::From, id)
        || tcx.is_diagnostic_item(sym::TryFrom, id)
        // A `&mut` to what a type keeps, called on the type itself (ADR 0169).
        || tcx.is_diagnostic_item(Symbol::intern("AsMut"), id)
        || tcx.is_diagnostic_item(Symbol::intern("BorrowMut"), id)
        // Never called: a map keys by value (ADR 0168).
        || is_std_def(tcx, id, StdItem::Hash)
        // A collection of the crate's: what `for`, `collect()`, `extend`,
        // `sum()` and `product()` call (ADR 0160).
        || tcx.is_diagnostic_item(sym::IntoIterator, id)
        || tcx.is_diagnostic_item(sym::FromIterator, id)
        || is_extend(tcx, id)
        || is_sum_or_product(tcx, id)
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

/// An impl of `Hash`, derived or the crate's own: never lowered, as a map
/// keys by value and never calls `hash` (ADRs 0121, 0168).
pub(crate) fn is_hash_impl(tcx: TyCtxt<'_>, id: DefId) -> bool {
    matches!(tcx.def_kind(id), DefKind::Impl { of_trait: true })
        && is_std_def(
            tcx,
            tcx.impl_trait_ref(id)
                .instantiate_identity()
                .skip_normalization()
                .def_id,
            StdItem::Hash,
        )
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

/// A std item's path as std names it, `std::str::Chars`: a `#![no_std]`
/// crate's rustc names it `core::str::Chars`, or `alloc::..`, which every
/// path compared here would miss.
fn std_path(tcx: TyCtxt<'_>, id: DefId) -> String {
    let path = tcx.def_path_str(id);
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

/// core's `TryFromSliceError` (ADR 0189).
pub(crate) fn is_try_from_slice_error(tcx: TyCtxt<'_>, ty: Ty<'_>) -> bool {
    matches!(ty.kind(), ty::Adt(adt, _) if tcx.crate_name(adt.did().krate) == sym::core
        && tcx.item_name(adt.did()).as_str() == "TryFromSliceError")
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

pub(super) fn is_operator(tcx: TyCtxt<'_>, id: DefId) -> bool {
    [
        LangItem::Add,
        LangItem::Sub,
        LangItem::Mul,
        LangItem::Div,
        LangItem::Rem,
        LangItem::Neg,
        LangItem::Not,
        LangItem::BitAnd,
        LangItem::BitOr,
        LangItem::BitXor,
        LangItem::Shl,
        LangItem::Shr,
        LangItem::AddAssign,
        LangItem::SubAssign,
        LangItem::MulAssign,
        LangItem::DivAssign,
        LangItem::RemAssign,
        LangItem::BitAndAssign,
        LangItem::BitOrAssign,
        LangItem::BitXorAssign,
        LangItem::ShlAssign,
        LangItem::ShrAssign,
        LangItem::Index,
        LangItem::IndexMut,
        LangItem::Deref,
        LangItem::DerefMut,
    ]
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
        let unknown = matches!(
            ty.kind(),
            ty::Param(_)
                | ty::Alias(
                    _,
                    ty::AliasTy {
                        kind: ty::Projection { .. },
                        ..
                    }
                )
        );
        unknown && {
            let tr = ty::TraitRef::new(self.tcx, trait_id, [ty]);
            matches!(
                self.tcx.codegen_select_candidate(self.typing_env.as_query_input(tr)),
                Ok(rustc_middle::traits::ImplSource::Param(_))
            )
        }
    }

    pub(super) fn is_generic_iter(&self, ty: ty::Ty<'tcx>) -> bool {
        self.bounded_by(ty, sym::Iterator)
    }

    pub(super) fn is_lazy_iter(&self, ty: ty::Ty<'tcx>) -> bool {
        let ty = self.reveal(ty.peel_refs());
        // std's sources that may never end (ADR 0128).
        let endless = matches!(ty.kind(), ty::Adt(adt, _) if self.tcx.crate_name(adt.did().krate) == sym::core
            && ["std::iter::Repeat", "std::iter::RepeatWith", "std::iter::Successors", "std::iter::FromFn"]
                .contains(&std_path(self.tcx, adt.did()).as_str()));
        endless
            || self.range_kind(ty) == Some(RangeKind::From)
            || self.is_user_iterator(ty)
            || self.is_generic_iter(ty)
            || self.is_dyn_iter(ty)
            || matches!(ty.kind(), ty::Adt(_, args) if self.is_array_iter(ty) && args.types().any(|t| self.is_lazy_iter(t)))
    }
}

/// Is `id` std's, core's or alloc's?
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
            let instance = ty::Instance::try_resolve(tcx, self.typing_env, method, method_args).ok()??;
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
        let instance = ty::Instance::try_resolve(self.tcx, self.typing_env, method, args).ok()??;
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

impl<'a, 'tcx> Recognition<'a, 'tcx> {
    pub(super) fn is_mutable_map_get(&self, id: DefId, args: ty::GenericArgsRef<'tcx>) -> bool {
        self.classify(id, args) == Some(Std::Map(MapOp::Get)) && self.tcx.item_name(id).as_str() == "get_mut"
    }
}

/// Is a struct's value its fields? The crate's own are, and std's
/// `PhantomData` and `Reverse(x)`, `[x]`; any other of std's has fields of
/// its own that aren't the JS value rust-js makes of it.
pub(super) fn struct_is_its_fields(tcx: TyCtxt<'_>, id: DefId) -> bool {
    ![sym::std, sym::core, sym::alloc].contains(&tcx.crate_name(id.krate))
        || tcx.is_lang_item(id, LangItem::PhantomData)
        || tcx.item_name(id).as_str() == "Reverse"
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

/// `dyn Iterator`, behind a reference or a `Box`, or not.
pub(super) fn is_dyn_iter<'tcx>(tcx: TyCtxt<'tcx>, ty: Ty<'tcx>) -> bool {
    let ty = ty.peel_refs();
    let ty = ty.boxed_ty().unwrap_or(ty);
    matches!(ty.kind(), ty::Dynamic(traits, ..)
        if traits.principal_def_id().is_some_and(|id| tcx.is_diagnostic_item(sym::Iterator, id)))
}

/// What `size_of`, `align_of` and `type_name` tell of a type (ADR 0145).
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Debug)]
pub(crate) enum TypeFact {
    Size,
    Align,
    Name,
}

/// The fact `def_id` asks of its type argument: `size_of` and `size_of_val`
/// its size, `align_of` its alignment, `type_name` and `type_name_of_val`
/// its name. None of any other function.
pub(crate) fn type_fact(tcx: TyCtxt<'_>, def_id: DefId) -> Option<TypeFact> {
    let diagnostic = |name: &str| tcx.is_diagnostic_item(Symbol::intern(name), def_id);
    if diagnostic("mem_size_of") || diagnostic("mem_size_of_val") {
        return Some(TypeFact::Size);
    }
    if diagnostic("mem_align_of") {
        return Some(TypeFact::Align);
    }
    match std_path(tcx, def_id).as_str() {
        "std::any::type_name" | "std::any::type_name_of_val" => Some(TypeFact::Name),
        _ => None,
    }
}

/// Is `ty` a `fmt::Result` (ADR 0054)?
pub(crate) fn is_fmt_result_type<'tcx>(tcx: TyCtxt<'tcx>, ty: Ty<'tcx>) -> bool {
    let Some(display) = tcx.get_diagnostic_item(Symbol::intern("Display")) else {
        return false;
    };
    let fmt = tcx.associated_item_def_ids(display)[0];
    let result = tcx.fn_sig(fmt).skip_binder().skip_binder().output();
    tcx.erase_and_anonymize_regions(ty) == tcx.erase_and_anonymize_regions(result)
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
}

/// The answer of the `fmt::Result` method `def_id`, if it's one of these.
pub(super) fn fmt_result_answer(tcx: TyCtxt<'_>, def_id: DefId) -> Option<FmtResultAnswer> {
    Some(match tcx.item_name(def_id).as_str() {
        "unwrap" => FmtResultAnswer::Unwrap,
        "expect" => FmtResultAnswer::Expect,
        "is_ok" => FmtResultAnswer::Is(true),
        "is_err" => FmtResultAnswer::Is(false),
        _ => return None,
    })
}

/// The question `def_id` asks of a `Formatter`: `f.width()` and the like.
pub(super) fn formatter_query(tcx: TyCtxt<'_>, def_id: DefId) -> Option<FormatterQuery> {
    if !std_path(tcx, def_id).starts_with("std::fmt::Formatter") {
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
    ArrayIntoIter,
    AsRef,
    Atomic,
    Borrow,
    BTreeMap,
    BTreeSet,
    BinaryHeap,
    BoxNew,
    Cell,
    Debug,
    Default,
    Display,
    Eq,
    Error,
    FmtWrite,
    From,
    Hash,
    Into,
    IntoIterator,
    Iterator,
    LocalKey,
    MaybeUninit,
    Ord,
    PhantomData,
    RefCell,
    Result,
    SliceIter,
    ToString,
    TryFrom,
    Vec,
    VecDeque,
}

impl StdItem {
    /// Its diagnostic item's name in std.
    fn name(self) -> Symbol {
        match self {
            StdItem::Any => Symbol::intern("Any"),
            StdItem::ArrayIntoIter => Symbol::intern("ArrayIntoIter"),
            StdItem::AsRef => sym::AsRef,
            StdItem::Atomic => Symbol::intern("Atomic"),
            StdItem::Borrow => Symbol::intern("Borrow"),
            StdItem::BTreeMap => Symbol::intern("BTreeMap"),
            StdItem::BTreeSet => Symbol::intern("BTreeSet"),
            StdItem::BinaryHeap => Symbol::intern("BinaryHeap"),
            StdItem::BoxNew => Symbol::intern("box_new"),
            StdItem::Cell => Symbol::intern("Cell"),
            StdItem::Debug => Symbol::intern("Debug"),
            StdItem::Default => Symbol::intern("Default"),
            StdItem::Display => Symbol::intern("Display"),
            StdItem::Eq => sym::Eq,
            StdItem::Error => Symbol::intern("Error"),
            StdItem::FmtWrite => Symbol::intern("FmtWrite"),
            StdItem::From => sym::From,
            StdItem::Hash => sym::Hash,
            StdItem::Into => sym::Into,
            StdItem::IntoIterator => sym::IntoIterator,
            StdItem::Iterator => sym::Iterator,
            StdItem::LocalKey => Symbol::intern("LocalKey"),
            StdItem::MaybeUninit => Symbol::intern("MaybeUninit"),
            StdItem::Ord => sym::Ord,
            StdItem::PhantomData => Symbol::intern("PhantomData"),
            StdItem::RefCell => Symbol::intern("RefCell"),
            StdItem::Result => sym::Result,
            StdItem::SliceIter => Symbol::intern("SliceIter"),
            StdItem::ToString => Symbol::intern("ToString"),
            StdItem::TryFrom => sym::TryFrom,
            StdItem::Vec => sym::Vec,
            StdItem::VecDeque => Symbol::intern("VecDeque"),
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
/// Is `id` std's `FromStr`, which has no diagnostic item to know it by?
pub(crate) fn is_from_str(tcx: TyCtxt<'_>, id: DefId) -> bool {
    tcx.crate_name(id.krate) == sym::core
        && tcx.def_kind(id) == DefKind::Trait
        && std_path(tcx, id).ends_with("str::FromStr")
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

pub(crate) fn is_std_def(tcx: TyCtxt<'_>, id: DefId, item: StdItem) -> bool {
    tcx.is_diagnostic_item(item.name(), id)
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

/// A provided method of a std trait that rust-js's dictionary of it has,
/// which a `dyn` or generic code calls through it: `Error::source` (ADR
/// 0141), and `fmt::Write`'s `write_char` and `write_fmt` (ADR 0180).
pub(crate) fn in_std_dictionary(tcx: TyCtxt<'_>, id: DefId) -> bool {
    is_std_method(tcx, id, StdItem::Error, "source") || is_writer_default(tcx, id)
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
    tcx.item_name(id).as_str() == "pad" && std_path(tcx, id).starts_with("std::fmt::Formatter")
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
        _ if is_std_def(tcx, id, StdItem::BoxNew) => Some(PureStd::BoxNew),
        _ => None,
    }
}
