# How Rust behaves in rust-js

rust-js keeps Rust's own front end: a program rustc rejects is rejected, and
borrow checking, type checking and exhaustive `match`es are rustc's
([ADR 0001](../decisions/0001-reuse-rustc-front-end.md)). What changes is what
runs: JavaScript. This page says, for each part of Rust, what that JavaScript
does, and where it differs from native Rust.

Three rules hold everywhere:

1. **Where Rust's behavior has a translation, the JS behaves as Rust does.**
   Each case below is checked by a program run both ways, natively and as JS,
   and compared ([ADR 0017](../decisions/0017-differential-testing.md),
   [the corpus](../../test/corpus)).
2. **Where it differs on purpose, this page says so.** Each difference is a
   choice for JavaScript, made in a design decision linked beside it.
3. **Anything else is refused at compile time**, with
   `rust-js does not support … yet`, a source location and no output
   ([ADR 0006](../decisions/0006-errors-and-unsupported-features.md)). A refusal
   is never a wrong answer.

The design decisions in [decisions/](../decisions) record why each choice was
made, as it was made. A "not yet" there may have been lifted by a later
decision; this page is what's true now. The [roadmap's gaps
table](../../ROADMAP.md) lists the larger missing pieces.

## Contents

- [Numbers](#numbers)
- [Text](#text)
- [Formatting and printing](#formatting-and-printing)
- [Values, copying and mutation](#values-copying-and-mutation)
- [Collections and iterators](#collections-and-iterators)
- [Destructors](#destructors)
- [Panics and errors](#panics-and-errors)
- [Async, threads and time](#async-threads-and-time)
- [Tests](#tests)
- [The JavaScript boundary](#the-javascript-boundary)
- [Differences from native Rust, in one list](#differences-from-native-rust-in-one-list)

## Numbers

| Rust | JS | Notes |
|---|---|---|
| `i8` `i16` `i32` `u8` `u16` `u32` | number | kept in range after each operation: `x \| 0`, `x >>> 0`, `x & 255` ([0011](../decisions/0011-numbers.md)) |
| `usize` `isize` | number, **32 bits** | rustc checks programs for `wasm32-unknown-unknown`, so `usize::MAX`, `size_of` and `cfg`s agree ([0090](../decisions/0090-wasm32-front-end.md)) |
| `i64` `u64` `i128` `u128` | BigInt, `5n` | exact, wrapped with `BigInt.asIntN` to 64 or 128 bits ([0086](../decisions/0086-64-bit-integers.md), [0171](../decisions/0171-128-bit-integers.md)) |
| `NonZero<T>` | its number; `new` of `0` is `None` | ([0177](../decisions/0177-nonzero.md)) |
| `Duration` | its nanoseconds, a BigInt; `{:?}` is std's, `1.5s` | ([0188](../decisions/0188-duration.md)) |
| `Wrapping<T>` | `[x]`, its number's arithmetic wrapped, shown as its number | ([0175](../decisions/0175-wrapping.md)) |
| `f64` | number | |
| `f32` | number, each result rounded by `Math.fround` | printed with its own shortest digits ([0122](../decisions/0122-f32.md)) |
| `bool` | boolean | |

- **Overflow wraps, as in a release build.** `i32::MAX + 1` is
  `i32::MIN`, as with `-Coverflow-checks=off`; a native debug build would
  panic. This holds even though `cfg!(debug_assertions)` is true, and
  `-Coverflow-checks=on` doesn't change it. Overflow rustc can see at
  compile time is still rustc's error
  ([0011](../decisions/0011-numbers.md), [`wrapping`](../../test/corpus/wrapping.rs)).
- **Division and remainder panic as Rust's do,** in every build: by zero,
  and `MIN / -1`, with Rust's messages, `attempt to divide by zero`
  ([0012](../decisions/0012-panics-and-runtime-helpers.md)).
- **A shift by an amount known only at run time is masked** to the type's
  width, as in a release build.
- **Casts are Rust's:** an integer wraps or truncates; a float saturates,
  `300.7 as u8` is `255`, and NaN is `0`.
- **`parse` is Rust's grammar, with Rust's errors:** `+42`, `inf`, `1e400`,
  a leading space refused, `ParseIntError { kind: PosOverflow }`, found
  where Rust finds it, `"999x"` overflowing a `u8` before its `x`
  ([0063](../decisions/0063-text.md), [0154](../decisions/0154-number-methods.md)).
- **An integer is never JS's `-0`:** `-2 % 2` is `0`, and so is `-4 * 0`
  ([0064](../decisions/0064-numbers.md), [`remainder_sign`](../../test/corpus/remainder_sign.rs),
  [`product_sign`](../../test/corpus/product_sign.rs)).
- **Arithmetic is exact; library functions are JS's.** `+ - * /` and `sqrt`
  give Rust's answers, an `f32`'s included. `sin`, `exp`, `powf` and the
  others are JS's `Math`, which may differ from Rust's in the last bit;
  `powf`'s special cases are Rust's, `1.0.powf(f64::NAN)` being 1, where JS's
  `**` gives NaN ([0182](../decisions/0182-boundary-matrices.md)).
- **`size_of`, `align_of`** are wasm32's: a pointer is 4 bytes
  ([0145](../decisions/0145-type-facts.md)).

- **Methods are std's, by its steps and with its panics:** `clamp`,
  `from_str_radix`, `div_ceil`, `ilog10`, `isqrt`, `midpoint`, a float's
  `signum`, `fract` and `to_radians`
  ([0154](../decisions/0154-number-methods.md)), and an integer's `checked_*`,
  `wrapping_*`, `overflowing_*` and `saturating_*`, each of the exact result
  ([0155](../decisions/0155-integer-families.md)), its bits rotated and its
  bytes, little-endian natively, as wasm32's are
  ([0156](../decisions/0156-integer-bits-and-bytes.md)). A NaN's sign and
  payload aren't kept, so `is_sign_negative` of one is `false`.

What's refused of numbers is in [the limitations](limitations.md#numbers-and-text).

## Text

A `String`, a `&str` and a `Box<str>` are a JS string; a `char` is a
one-character string ([0023](../decisions/0023-strings-references-shared-state.md),
[0034](../decisions/0034-strings-and-chars.md)). JS strings can't be changed, so
sharing one is copying it: `clone()` is the string itself, and `push_str` is
`s += t`.

- **Lengths, slices and offsets count UTF-8 bytes, as Rust's do.**
  `"héllo".len()` is `6`; `&s[a..b]`, `find`, `rfind` and `char_indices`
  are in bytes; a slice inside a character panics with Rust's message,
  `byte index 2 is not a char boundary; it is inside 'é' (bytes 1..3 of
  string)` ([0138](../decisions/0138-string-byte-counts.md)). Each counts from
  the start of the string.
- **A `String` is edited by its bytes too:** `pop`, `insert`, `remove`,
  `truncate`, `retain` and `clear` give its place the new string, and panic
  where Rust's do ([0149](../decisions/0149-string-editing.md)).
- **`chars()` is by code point; `bytes()` and `as_bytes()` are the UTF-8
  bytes;** a byte string `b"GET"` is `[71, 69, 84]`
  ([0126](../decisions/0126-byte-strings.md)).
- **`to_uppercase` and `to_lowercase` are Unicode's,** `ß` to `SS`;
  `to_ascii_uppercase` changes ASCII letters only. `char`'s questions,
  `is_alphabetic` and the rest, are Unicode properties
  ([0063](../decisions/0063-text.md)). Their tables are the JS engine's, which
  may be another Unicode version than Rust's.
- **`split`, `lines`, `split_whitespace`, `strip_prefix`, `split_once`,
  `replace`, `starts_with`** and the other common methods behave as Rust's,
  with an empty pattern too. A closure, a function such as
  `char::is_numeric`, or a set of `char`s, `[';', ',']`, is a pattern of
  `find`, `rfind`, `split`, `contains`, `starts_with`, `ends_with` and the
  `trim_*_matches` ([0157](../decisions/0157-char-predicates.md)); `get(range)`
  is `None` where slicing panics. `splitn`, `rsplit`, `split_terminator`, `split_at`,
  `match_indices`, `matches` and the `trim_*_matches` search as Rust's do,
  `rsplit` from the end ([0150](../decisions/0150-string-patterns.md)).
- **Strings and `char`s order by code point,** `<`, `cmp`, `sort()`, a
  `BTreeMap`'s keys and a range of `char`s, an emoji after U+FFFD as in
  Rust; **`trim()` removes Unicode's White_Space,** U+0085 but not U+FEFF
  ([0183](../decisions/0183-code-point-order-and-trim.md)).

- **A type's own `FromStr` is what `parse` calls,** `s.parse::<Role>()`
  calling `impl FromStr for Role`'s `from_str`
  ([0159](../decisions/0159-user-from-str.md)); a generic `T: FromStr`'s is
  its dictionary's, std's or the crate's ([0161](../decisions/0161-generic-from-str.md)).

- **A `Path` or a `PathBuf` is its text,** made from text and shown by
  `display()`; its `==` compares components, as Rust's does
  ([0173](../decisions/0173-paths-as-text.md)). A parse error's `kind()` is
  its kind, and `{:?}` of std's `IntErrorKind` its name.
- **Bytes become text as Rust checks UTF-8:** `str::from_utf8`'s
  `Utf8Error` says where the first bad sequence starts and how long it is,
  and `from_utf8_lossy` gives a U+FFFD for each
  ([0172](../decisions/0172-utf8-decoding.md)).

What's refused of text is in [the limitations](limitations.md#numbers-and-text).

## Formatting and printing

`format!` is a template literal; `println!` is `console.log`, `eprintln!`
`console.error`, and `print!` writes without a newline
([0087](../decisions/0087-printing.md), [0132](../decisions/0132-std-odds.md)).

- **Every placeholder option is Rust's:** width, fill, alignment, sign, `#`,
  `0`, precision, `{:x}`, `{:b}`, `{:o}`, `{:e}`, positional and named
  arguments ([0058](../decisions/0058-format-options.md)).
- **Floats print Rust's shortest digits:** `{}` never uses an exponent,
  `{:?}` of `1.0` is `1.0`, and `{:.0}` of `2.5` is `2`
  ([0064](../decisions/0064-numbers.md)).
- **`{:?}` is Rust's `Debug`:** a type's name and fields, a string's
  escapes, `{:#?}` pretty ([0060](../decisions/0060-debug.md),
  [0137](../decisions/0137-pretty-debug.md)).
- **A type's own `Display` and `Debug` are given the placeholder's
  options,** and can ask its `Formatter` for them, `f.width()`, `f.pad(s)`
  ([0143](../decisions/0143-formatter-options.md)).
- **`write!` and `writeln!` into a `String` are `s += ..`,** and the
  `fmt::Result` they give is `Ok` but where what they write fails
  ([0148](../decisions/0148-write-to-string.md)).
- **A writer's `Err(fmt::Error)`,** as chrono's formatting returns, goes up
  through each `?` with what was written before it: a `write!` to a
  `String` gives it back, and `to_string()`, `format!` and `println!` panic
  as std's do ([0187](../decisions/0187-fmt-error.md)).
- **Arguments are taken in Rust's order:** `println!("{} {:?}", v.len(),
  v.pop())` reads the length before the pop
  ([0034](../decisions/0034-strings-and-chars.md)).
- **`{:x}`, `{:e}` and `{:p}` of the crate's types call its own `LowerHex`,
  `LowerExp`, `Pointer` and the like,** given the placeholder's options
  ([0165](../decisions/0165-other-fmt-traits.md)).
- **A writer of the crate's, `impl fmt::Write`, is given its text whole:**
  one `write_str` for a `write!`, where Rust's gives it a piece at a time
  ([0166](../decisions/0166-user-fmt-write.md)).
- **Generic code that writes to any writer, a `W: fmt::Write`,** is given
  a `String`, a `Formatter` or the crate's own writer in a box, with a
  dictionary of its `write_str`, `write_char` and `write_fmt`
  ([0180](../decisions/0180-generic-writers.md)).

What's refused of formatting is in [the limitations](limitations.md#formatting).

## Values, copying and mutation

| Rust | JS |
|---|---|
| struct | object, `{ x, y }` ([0020](../decisions/0020-structs-and-tuples.md)) |
| tuple, tuple struct | array, `[a, b]` |
| unit struct, `()` | `undefined`; a unit struct named `#[rust_js::name = "click"]`, that string ([0223](../decisions/0223-webapi-event-and-tag-maps.md)) |
| fieldless enum variant | its name, `"Green"` ([0013](../decisions/0013-fieldless-enums.md)) |
| enum variant with fields | `{ TAG: "Circle", r }`, `{ TAG: "Some", _0: x }` ([0033](../decisions/0033-enums-with-fields.md)) |
| `Result` | `{ TAG: "Ok", _0: v }`, `{ TAG: "Err", _0: e }` ([0035](../decisions/0035-results-and-throwing-js.md)) |
| `Option` | the value, or `undefined` for `None` ([0030](../decisions/0030-option.md)); `null` for `None` of a `#[rust_js::nullable]` field or function ([0275](../decisions/0275-nullable-writes-null.md), [0574](../decisions/0574-what-reacts-scheduler-heap-asks.md)); a `Some` of what could look like `None`, `Option<()>` or `Option<Option<T>>`, is boxed, `{ $someNone: n }` ([0051](../decisions/0051-generic-options.md)) |
| `#[repr(transparent)]` struct | its field: `Meters(x)` is `x`, `m.0` is `m` ([0584](../decisions/0584-a-transparent-struct-is-its-field.md)) |
| `Box<T>`, `Arc<T>` | the value itself |
| `Rc<T>` | the value itself; `{ value, strong, weak }` where the crate reads its counts, `strong_count` or `Weak` ([0320](../decisions/0320-counted-rcs.md)) |
| `Cell`, `RefCell`, `Mutex`, `RwLock`, an atomic, `static mut` | `{ value }`; a `Cell`, or a `RefCell` only borrowed for a moment, in a struct's field or a variable, is its value ([0144](../decisions/0144-locks.md), [0287](../decisions/0287-cells-as-variables.md), [0288](../decisions/0288-cell-fields.md), [0362](../decisions/0362-refcell-fields-never-counted.md)) |
| `thread_local!` | a variable of its module, `let taskCount`, or a `{ value }` cell (below) ([0270](../decisions/0270-module-variables.md)) |
| `Vec`, slice, array | array |

- **A move never copies:** Rust won't let the old name be used again.
- **A `Copy` value is copied only if its type is changed in place
  somewhere,** `{ ...p }`; otherwise sharing the object can't be told apart
  from copying it. A derived `clone()` copies what needs copying, and
  `vec![x; n]` makes `n` separate values
  ([0052](../decisions/0052-std-trait-impls.md),
  [`copy_mutation`](../../test/corpus/copy_mutation.rs)).
- **`&T` is the value. `&mut` to an object is the object**, changed in place
  ([0025](../decisions/0025-vec-loops-refcell-mut.md)). A `&mut` to a number, a
  string or an `Option` names its place when it's in a variable, `*r += 1`
  being `x += 1`; given to a function, it's a box `{ value }` the caller
  reads back; kept elsewhere, a handle with a getter and a setter
  ([0074](../decisions/0074-mut-boxes.md),
  [0099](../decisions/0099-mut-references.md)).
- **`*r = v` replaces the value whole, as Rust's does:** an object `r` is
  becomes `v` in place, `$assign(r, v)`, so every name for it sees `v`;
  `mem::replace`, `mem::swap` and `mem::take` the same. An enum with a
  fieldless variant, whose `Off` is a string, is reached through its place
  instead, as a number is ([0147](../decisions/0147-replacing-through-mut.md)).
- **`Rc::clone` shares the one object,** as Rust's does. Where the crate
  reads an `Rc`'s counts, `Rc::strong_count` is `rc.strong`, counted where
  it's made, cloned and dropped, and the last drop runs what it points at's
  destructor ([0320](../decisions/0320-counted-rcs.md)). `Rc::ptr_eq` of
  an `Rc` of a struct with fields is `a === b` of the objects, and counts
  nothing ([0582](../decisions/0582-ptr-eq-of-an-object-is-its-identity.md)).
- **A `&mut` std gives to a number or a string in a collection is a
  handle on it,** where it's used as a value: `iter_mut()`, `values_mut()`,
  `get_mut`, `last_mut()` and a `for` over `&mut m`, each writing the slot
  it's of ([0152](../decisions/0152-std-item-handles.md)).
- **A `RefCell` counts its borrows where anything could ask,** as std's
  does: a `borrow_mut()` while another borrow is held throws `RefCell
  already borrowed` where Rust panics, and a held guard is released where
  Rust drops it, in a `finally`. A borrow nothing can meet while it's held
  is checked and not counted, and a `RefCell` field only borrowed for a
  moment, by a statement's `borrow_mut()`, `replace` or `take`, is its
  value, read and written
  ([0328](../decisions/0328-refcell-borrow-counts.md),
  [0580](../decisions/0580-a-call-asks-of-a-cell-only-what-it-may-borrow.md),
  [0585](../decisions/0585-a-refcells-replace-holds-it-for-a-moment.md)):

```rust
# use std::cell::RefCell;
#
pub struct Task {
    pub id: u32,
    pub callback: RefCell<Option<Box<dyn FnMut(bool) -> u32>>>,
}

pub fn run(task: &Task) -> u32 {
    match task.callback.take() {
        Some(mut f) => {
            let n = f(false);
            task.callback.replace(Some(f));
            n
        }
        None => 0,
    }
}
```

```js
export function run(task) {
  const old = task.callback;
  task.callback = undefined;
  if (old != null) {
    const n = old(false);
    task.callback = old;
    return n;
  }
  return 0;
}
```

- **A `Mutex` and an `RwLock` are counted as a `RefCell` is.** A lock
  locked again while its thread holds it, which deadlocks in Rust, throws
  `rust-js does not support locking a lock its thread holds, which
  deadlocks in Rust` ([0328](../decisions/0328-refcell-borrow-counts.md)).
- **A `thread_local!` is its module's variable,** named as JS names one,
  camelCase of its words, `TASK_QUEUE` as `taskQueue`, while a `const`
  keeps its name, `MAX_SIZE` ([0270](../decisions/0270-module-variables.md),
  [0579](../decisions/0579-a-thread-local-is-named-as-a-variable.md)). Its
  `with_borrow_mut(|queue| ..)` is its closure's body, reading the variable
  in place ([0581](../decisions/0581-a-closure-called-as-a-statement-is-its-body.md)):

```rust
# use std::cell::RefCell;
#
thread_local! {
    static TASK_QUEUE: RefCell<Vec<u32>> = const { RefCell::new(Vec::new()) };
}

pub fn push_task(id: u32) {
    TASK_QUEUE.with_borrow_mut(|queue| queue.push(id));
}

pub fn queued() -> usize {
    TASK_QUEUE.with_borrow(Vec::len)
}
```

```js
const taskQueue = [];

export function pushTask(id) {
  taskQueue.push(id);
}

export function queued() {
  return taskQueue.length;
}
```

Where what `with_borrow` runs, a closure or a function such as
`Vec::len`, may borrow the same thread-local again, through a function
of the crate's that does, the thread-local is its cell, `{ value: [] }`,
borrowed while it runs, `$withBorrowMut(log, (log) => ..)`, and a second
borrow throws as Rust's panics
([0580](../decisions/0580-a-call-asks-of-a-cell-only-what-it-may-borrow.md),
[0587](../decisions/0587-a-function-given-to-with-borrow-is-asked-as-a-closure.md)).
So is one handed out by `with(|cell| ..)`, or read by another module.

Differences:

- **A lock is never poisoned:** a panic that unwinds through a held guard
  releases it, and JS that catches the panic and calls back into Rust finds
  the lock as if nothing had happened, where Rust's would be poisoned
  ([0144](../decisions/0144-locks.md)).
- **A JS getter that calls back into Rust while a cell is borrowed isn't
  seen:** where a binding reads a property, rust-js takes it to run no
  Rust, so a borrow such a getter makes of a held cell passes, where
  Rust's would panic. No binding rust-js ships does it
  ([0580](../decisions/0580-a-call-asks-of-a-cell-only-what-it-may-borrow.md)).

What's refused is in [the limitations](limitations.md).

## Collections and iterators

| Rust | JS |
|---|---|
| `Vec`, `VecDeque` | array ([0068](../decisions/0068-queues.md)) |
| `BinaryHeap` | array, in Rust's heap order: ties pop as Rust's do |
| `HashMap`, `HashSet` | `Map`, `Set` ([0059](../decisions/0059-hashmap.md)); keyed by value for a struct, tuple or enum key with a derived `Eq`, `$KeyMap`, whatever its `Hash` ([0121](../decisions/0121-value-keys.md), [0168](../decisions/0168-user-hash.md)) |
| `BTreeMap`, `BTreeSet` | `Map`, `Set`, iterated in key order |

- **An iterator chain is an array's methods,** `v.map(f).filter(p)`, when
  its closures do nothing anyone can see. When one prints, changes
  something or may panic, and something after it could tell, the chain is a
  lazy JS iterator, run one item at a time in Rust's order
  ([0036](../decisions/0036-iterators-and-sorting.md),
  [0139](../decisions/0139-lazy-chains.md)).
- **A shared slice is a copy,** `&v[a..b]`, `split_at` and `get(a..b)`:
  nothing changes `v` while it's borrowed
  ([0153](../decisions/0153-collection-and-cell-methods.md)). **A `&mut`
  slice is a view of its array,** `$view(v, 1)`, whose writes are the
  array's ([0335](../decisions/0335-slice-views.md)).
- **`starts_with` and `ends_with` of a slice compare items as `==` does,**
  by `===` of numbers, strings and `bool`s, and by the items' own
  `PartialEq` of the rest ([0336](../decisions/0336-slice-prefixes-by-eq.md)).
- **An `impl Iterator` is the iterator it stands for**
  ([0061](../decisions/0061-generic-iterators.md)).
- **A clone of std's iterator over an array is what's left of the array,**
  its owned items copied where they need it
  ([0181](../decisions/0181-iterator-clones.md)).
- **A collection of the crate's is its own impls':** `for` over it calls
  its `IntoIterator`, `collect()` its `FromIterator`, `extend` its
  `Extend`, `sum()` its `Sum` ([0160](../decisions/0160-user-collections.md)).
  Given where any `IntoIterator` goes, to `v.extend(cart)` or generic code,
  it's refused: that code wouldn't call its `into_iter`. An iterator of the
  crate's runs from both ends by its own `next_back`, `rev()` included, and
  its `len()` is its `size_hint()`, checked ([0164](../decisions/0164-double-ended-iterators.md)).
  `size_hint()` is Rust's where it's known: std's `(0, None)` of one of the
  crate's, a std `ExactSizeIterator`'s length ([0170](../decisions/0170-size-hint.md)).
- **A function passed where a closure goes is the arrow that calls it,**
  `.map(str::len)` being `.map((s) => $byteLen(s))` and `fold(0, i32::max)`
  being `reduce((a, b) => Math.max(a, b), 0)`; a constructor's makes what
  its call makes ([0125](../decisions/0125-function-values.md),
  [0151](../decisions/0151-called-function-values.md)), one taking a
  `&mut` included, `iter_mut().for_each(bump)`.

Differences:

- **A `HashMap` or a `HashSet` iterates in insertion order,** where Rust's
  order is unspecified; serde_json writes one in that order too.

What's refused of collections and iterators is in
[the limitations](limitations.md#iterators-and-collections).

## Destructors

`Drop` runs where Rust runs it: in reverse order of declaration, not for a
value moved away, on reassignment, at the end of a temporary, on an early
`return`, and while a panic unwinds, each through a `try`/`finally`. Code
with no `Drop` impl has no extra JS ([0098](../decisions/0098-destructors.md)).
A `Drop` is its body where it's dropped, at each drop, its guard its
fields, `const previous = depth; try { .. } finally { depth = previous; }`
([0589](../decisions/0589-a-drop-dropped-once-is-its-body-there.md)).

Differences: a value with a destructor that JS holds, given out by an
exported generic function, is never dropped; a `drop` that panics while
another panic unwinds replaces that panic, where Rust aborts.

Generic code drops a value of an associated type through its impl's
dictionary ([0178](../decisions/0178-associated-type-drops.md)).
`Option::filter` drops what it doesn't keep, and `map_or` its unused
fallback ([0179](../decisions/0179-option-filter-map-or-drops.md)).
Generic code that loops over or folds an iterator of a type parameter
takes no destructor of it: a call that gives one is refused, at the call
([0190](../decisions/0190-generic-no-destructor.md)).
An or-pattern moves what each alternative moves, where a variant tells
them apart, and a temporary in an arm's body ends with the arm
([0191](../decisions/0191-or-patterns-arm-temporaries-boxed-str.md)).

What's refused of destructors is in [the limitations](limitations.md#types).

## Panics and errors

A panic is `throw new Error(message)`, with Rust's message: `panic!`,
`unwrap()` of `None` or of an `Err` (with the error's own `{:?}`),
`expect`, an index out of bounds, an `assert_eq!` with both sides, a map's
missing key, `unreachable!`, `todo!` ([0012](../decisions/0012-panics-and-runtime-helpers.md),
[0026](../decisions/0026-testing.md)).

- **`Result` and `?` are Rust's:** `?` returns early, through the crate's
  own `From` impls; `Box<dyn Error>` takes the crate's errors, strings,
  `"msg".into()`, std's parse errors and serde_json's
  ([0035](../decisions/0035-results-and-throwing-js.md),
  [0141](../decisions/0141-std-trait-objects.md)).
- **Control flow is Rust's:** `loop` with a value, labeled `break` and
  `continue`, labeled blocks ([0158](../decisions/0158-labeled-blocks.md)),
  `let`-`else`, `if let` chains, match guards, `let` guards.

Differences:

- **A crate is a library: nothing calls `main`.** A host calls it, and an
  uncaught panic is the host's error: a JS stack trace and exit code 1,
  where Rust prints `thread 'main' panicked at` and exits with 101. A
  `main` that returns `Err` returns it to its caller.
- **JS can catch a panic,** an ordinary `Error`; Rust code can't
  (`catch_unwind` is refused).

What's refused of panics and processes is in
[the limitations](limitations.md#panics-processes-and-io).

## Async, threads and time

`async fn` is an `async function`, `.await` is `await`, and an `async move`
block or an `async` closure is an async arrow
([0029](../decisions/0029-async-await.md)). A JS caller of one gets a
`Promise`.

- **A future starts when it's made,** runs to its first `await` at once,
  and runs to its end though nothing awaits it, as a JS promise does.
  Rust's waits until it's polled. This is what lets JS call an `async fn`
  and get an ordinary promise, with no Rust executor.
- **`js::spawn` starts a future no one awaits;** `js::settle(p).await`
  makes a rejected promise an `Err` ([0102](../decisions/0102-js-and-webapi.md)).
- **There's one thread.** `std::thread::spawn` is refused; a channel is a
  queue its ends share, and `recv()` of an empty one whose senders are
  alive panics, where Rust's would wait forever
  ([0142](../decisions/0142-channels.md)).
- **The time is JS's clocks'.** A `SystemTime` and an `Instant` are their
  nanoseconds, a BigInt; `SystemTime::now()` is `Date.now()`'s, to the
  millisecond, and `Instant::now()` is `performance`'s, which only goes
  forward ([0447](../decisions/0447-the-clock.md)).

What's refused of async, threads and time is in
[the limitations](limitations.md#async-threads-and-time).

## Tests

`rust-js --test` writes a crate's `#[test]` functions and a `.test.js`
that registers each, for `bun test` or a browser runner
([0026](../decisions/0026-testing.md), [0027](../decisions/0027-real-browser-tests.md)).
`#[should_panic]` and `#[ignore]` are libtest's, assertion messages are
Rust's, and a test returning a `Result` fails on `Err`. A struct error in
that failure shows without its type's name.

## The JavaScript boundary

**A crate is a library of ES modules,** one per Rust module, `mod shapes`
being `shapes.js`, or `.jsx` with JSX in it
([0019](../decisions/0019-one-js-file-per-module.md),
[0073](../decisions/0073-named-module-imports.md)). Each imports the runtime
helpers it uses from `@rust-js/runtime`, which must be the compiler's
version ([0103](../decisions/0103-runtime-package.md)).

What JS sees of a crate:

- **A `pub fn` is an `export function` of its name in camelCase**, as
  every Rust name is in JS: `first_word` is `firstWord`, a hook
  `use_clicks` `useClicks`, a field `sort_index` `sortIndex`, and a name
  JS reserves gets a `$`, `delete$`
  ([0038](../decisions/0038-js-names-and-destructuring.md),
  [0573](../decisions/0573-every-rust-name-is-camel-case-in-js.md)). A
  `thread_local!` is camelCase of its words, `TASK_QUEUE` as `taskQueue`,
  and a `const` is as written, `MAX_SIZE`
  ([0579](../decisions/0579-a-thread-local-is-named-as-a-variable.md)). A
  name that isn't the camelCase one is `#[rust_js::name = ".."]`'s, the one
  way to name an item in JS.
- **A type's methods are its exported object's,** the receiver first:
  `Point.shift(p, by)` ([0047](../decisions/0047-methods.md)).
- **`LowerHex`, `Pointer` and the other formatting traits are
  dictionaries too:** `{:x}` of a generic `T` is its `fmt`, given the
  placeholder's options ([0174](../decisions/0174-generic-fmt-traits.md)).
- **`FromStr`, `AsRef` and `Borrow` are dictionaries too:** a generic
  `S: AsRef<str>` is given `{ asRef }`, a `K: Borrow<Q>` `{ borrow }`,
  the value itself for a `String` or a `&str`
  ([0161](../decisions/0161-generic-from-str.md),
  [0162](../decisions/0162-generic-as-ref.md), [0167](../decisions/0167-borrow.md)).
- **A generic function takes its trait dictionaries after its arguments,**
  `show(t, pointDisplay)`; a `dyn Trait` is `{ value, impl }`
  ([0049](../decisions/0049-traits-and-generics.md)).
- **Values are the shapes above:** a struct an object literal, a `u64` a
  BigInt, `None` `undefined` or `null`, a `&mut` to a number `{ value }`, a
  closure a function, an `async fn`'s result a `Promise`.
- **A `#[rust_js::nullable]` field's or function's `None` is `null`,**
  where it's written, as a JS caller testing `!== null` expects
  ([0275](../decisions/0275-nullable-writes-null.md),
  [0574](../decisions/0574-what-reacts-scheduler-heap-asks.md),
  [0578](../decisions/0578-a-nullable-result-where-its-written.md)):

```rust
#[cfg_attr(rust_js, rust_js::nullable)]
pub fn first(v: &[u32]) -> Option<u32> {
    v.first().copied()
}
```

```js
export function first(v) {
  return v[0] ?? null;
}
```

On a field or a function whose type isn't an `Option`, it's a compile
error that says so, `` `#[rust_js::nullable]` of a function is of its
result, which isn't an `Option` ``
([0586](../decisions/0586-a-nullable-field-is-an-option.md)).

- **A `pub use` of another module's item re-exports it,** `pub use
  shapes::area` being `export { area } from "./shapes.js";`
  ([0240](../decisions/0240-reexports.md)).
- **A crate's JS is shaped by the compiler that wrote it:** a JS app
  imports a library's JS as its author's `rust-js build` wrote it, with
  its TypeScript declarations
  ([0196](../decisions/0196-typescript-declarations.md),
  [0414](../decisions/0414-a-library-on-npm-as-rescripts-are.md)).

Calling JavaScript:

- **A binding is a Rust function with a `link_name`,**
  `#[cfg_attr(rust_js, rust_js::link_name = "..")]`: a global, a method
  (`this` first), a property, `new X`, an import from a module
  ([0021](../decisions/0021-js-interop.md), [0028](../decisions/0028-js-module-imports.md),
  [0039](../decisions/0039-generic-bindings.md),
  [0373](../decisions/0373-one-binding-form.md)). A JS global or an
  import with no body to write is a `static` in an `unsafe extern "Rust"`
  block, `#[link_name = ".."]`. `js::import!("./app.css")` imports a file
  for its effects ([0110](../decisions/0110-stable-syntax.md)).
- **A global is its own type, as TypeScript declares it**, read as its
  name. Whether the host has it is `js::is_defined(binding)`, `typeof X
  !== "undefined"`; a binding called where the host lacks it throws a
  `ReferenceError`, as JS does
  ([0597](../decisions/0597-whether-the-host-has-a-global-is-is-defined.md)):

```rust
pub fn has_set_immediate() -> bool {
    js::is_defined(js::set_immediate)
}
```

```js
export function hasSetImmediate() {
  return typeof setImmediate !== "undefined";
}
```

- **The `js` crate binds JS's own globals, the `webapi` crate the
  browser's,** generated from its WebIDL; options objects are structs of
  `Option`s ([0102](../decisions/0102-js-and-webapi.md)). What each crate
  rust-js ships binds is [Libraries](libraries/README.md). A library's
  binding is an npm package ([0118](../decisions/0118-bindings-on-npm-only.md)).
- **A binding is trusted as declared.** One returning
  `Result<T, &JsError>` makes what JS throws an `Err`; any other lets it
  propagate, as a panic. Declare `Option` where JS may give `null`.
- **A value of unknown shape is a `js::Unknown`**, which `js::classify`
  tells by `typeof` and `js::get` reads a property of by name, `value[key]`
  ([0225](../decisions/0225-unknown-values.md)).

JSON through serde_json writes and reads as serde_json does, a 64-bit
integer to the digit; `rename`, `tag`, `untagged`, `flatten`, `default`,
`skip` and the other common attributes are supported
([0077](../decisions/0077-serde-json.md)–[0083](../decisions/0083-serde-json-value.md)).
What's refused of serde is in [the limitations](limitations.md#serde).
React and JSX are [their own pages](README.md#react).

## Differences from native Rust, in one list

1. Integer overflow wraps, as a release build's does, in every build.
2. `usize` and `isize` are 32 bits.
3. `i64` and `u64` are BigInts at the JS boundary.
4. Float library functions, `sin`, `exp`, `powf`, are JS's: the last bit may
   differ. A NaN's sign and payload aren't kept.
5. A `HashMap` iterates in insertion order.
6. A lock is never poisoned; one locked again by its thread throws, where
   Rust deadlocks.
7. A JS getter that calls back into Rust and borrows a held cell isn't
   seen.
8. A future starts when it's made, and runs though no one awaits it.
9. There's one thread: a channel's `recv()` that would wait forever panics.
10. Nothing calls `main`; an uncaught panic is the host's error, exit code 1.
11. JS can catch a panic; a `drop` panicking during a panic replaces it.
12. A value with a destructor that JS holds is never dropped.
13. A writer of the crate's is given each `write!`'s text in one `write_str`.
14. A generic iterator's `size_hint()` is its array's length, or `(0, None)`.
