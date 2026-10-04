# How Rust behaves in rust-js

rust-js keeps Rust's own front end: a program rustc rejects is rejected, and
borrow checking, type checking and exhaustive `match`es are rustc's
([ADR 0001](decisions/0001-reuse-rustc-front-end.md)). What changes is what
runs: JavaScript. This page says, for each part of Rust, what that JavaScript
does, and where it differs from native Rust.

Three rules hold everywhere:

1. **Where Rust's behavior has a translation, the JS behaves as Rust does.**
   Each case below is checked by a program run both ways, natively and as JS,
   and compared ([ADR 0017](decisions/0017-differential-testing.md),
   [the corpus](../test/corpus)).
2. **Where it differs on purpose, this page says so.** Each difference is a
   choice for JavaScript, made in a design decision linked beside it.
3. **Anything else is refused at compile time**, with
   `rust-js does not support … yet`, a source location and no output
   ([ADR 0006](decisions/0006-errors-and-unsupported-features.md)). A refusal
   is never a wrong answer.

The design decisions in [decisions/](decisions) record why each choice was
made, as it was made. A "not yet" there may have been lifted by a later
decision; this page is what's true now. The [roadmap's gaps
table](../ROADMAP.md) lists the larger missing pieces.

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
| `i8` `i16` `i32` `u8` `u16` `u32` | number | kept in range after each operation: `x \| 0`, `x >>> 0`, `x & 255` ([0011](decisions/0011-numbers.md)) |
| `usize` `isize` | number, **32 bits** | rustc checks programs for `wasm32-unknown-unknown`, so `usize::MAX`, `size_of` and `cfg`s agree ([0090](decisions/0090-wasm32-front-end.md)) |
| `i64` `u64` | BigInt, `5n` | exact, wrapped with `BigInt.asIntN` ([0086](decisions/0086-64-bit-integers.md)) |
| `f64` | number | |
| `f32` | number, each result rounded by `Math.fround` | printed with its own shortest digits ([0122](decisions/0122-f32.md)) |
| `bool` | boolean | |
| `i128` `u128` | refused | |

- **Overflow wraps, as in a release build.** `i32::MAX + 1` is
  `i32::MIN`, as with `-Coverflow-checks=off`; a native debug build would
  panic. This holds even though `cfg!(debug_assertions)` is true, and
  `-Coverflow-checks=on` doesn't change it. Overflow rustc can see at
  compile time is still rustc's error
  ([0011](decisions/0011-numbers.md), [`wrapping`](../test/corpus/wrapping.rs)).
- **Division and remainder panic as Rust's do,** in every build: by zero,
  and `MIN / -1`, with Rust's messages, `attempt to divide by zero`
  ([0012](decisions/0012-panics-and-runtime-helpers.md)).
- **A shift by an amount known only at run time is masked** to the type's
  width, as in a release build.
- **Casts are Rust's:** an integer wraps or truncates; a float saturates,
  `300.7 as u8` is `255`, and NaN is `0`.
- **`parse` is Rust's grammar, with Rust's errors:** `+42`, `inf`, `1e400`,
  a leading space refused, `ParseIntError { kind: PosOverflow }`, found
  where Rust finds it, `"999x"` overflowing a `u8` before its `x`
  ([0063](decisions/0063-text.md), [0154](decisions/0154-number-methods.md)).
- **An integer is never JS's `-0`:** `-2 % 2` is `0`
  ([0064](decisions/0064-numbers.md), [`remainder_sign`](../test/corpus/remainder_sign.rs)).
- **Arithmetic is exact; library functions are JS's.** `+ - * /` and `sqrt`
  give Rust's answers, an `f32`'s included. `sin`, `exp`, `powf` and the
  others are JS's `Math`, which may differ from Rust's in the last bit.
- **`size_of`, `align_of`** are wasm32's: a pointer is 4 bytes
  ([0145](decisions/0145-type-facts.md)).

- **Methods are std's, by its steps and with its panics:** `clamp`,
  `from_str_radix`, `div_ceil`, `ilog10`, `isqrt`, `midpoint`, a float's
  `signum`, `fract` and `to_radians`
  ([0154](decisions/0154-number-methods.md)), and an integer's `checked_*`,
  `wrapping_*`, `overflowing_*` and `saturating_*`, each of the exact result
  ([0155](decisions/0155-integer-families.md)), its bits rotated and its
  bytes, little-endian natively, as wasm32's are
  ([0156](decisions/0156-integer-bits-and-bytes.md)). A NaN's sign and
  payload aren't kept, so `is_sign_negative` of one is `false`.

Refused: `i128` and `u128`; `mul_add`; a float's `midpoint`; and some
integer and float methods, among them `swap_bytes`, `reverse_bits` and
`copysign`.

## Text

A `String`, a `&str` and a `Box<str>` are a JS string; a `char` is a
one-character string ([0023](decisions/0023-strings-references-shared-state.md),
[0034](decisions/0034-strings-and-chars.md)). JS strings can't be changed, so
sharing one is copying it: `clone()` is the string itself, and `push_str` is
`s += t`.

- **Lengths, slices and offsets count UTF-8 bytes, as Rust's do.**
  `"héllo".len()` is `6`; `&s[a..b]`, `find`, `rfind` and `char_indices`
  are in bytes; a slice inside a character panics with Rust's message,
  `byte index 2 is not a char boundary; it is inside 'é' (bytes 1..3 of
  string)` ([0138](decisions/0138-string-byte-counts.md)). Each counts from
  the start of the string.
- **A `String` is edited by its bytes too:** `pop`, `insert`, `remove`,
  `truncate`, `retain` and `clear` give its place the new string, and panic
  where Rust's do ([0149](decisions/0149-string-editing.md)).
- **`chars()` is by code point; `bytes()` and `as_bytes()` are the UTF-8
  bytes;** a byte string `b"GET"` is `[71, 69, 84]`
  ([0126](decisions/0126-byte-strings.md)).
- **`to_uppercase` and `to_lowercase` are Unicode's,** `ß` to `SS`;
  `to_ascii_uppercase` changes ASCII letters only. `char`'s questions,
  `is_alphabetic` and the rest, are Unicode properties
  ([0063](decisions/0063-text.md)). Their tables are the JS engine's, which
  may be another Unicode version than Rust's.
- **`split`, `lines`, `split_whitespace`, `strip_prefix`, `split_once`,
  `replace`, `starts_with`** and the other common methods behave as Rust's,
  with an empty pattern too. A closure, a function such as
  `char::is_numeric`, or a set of `char`s, `[';', ',']`, is a pattern of
  `find`, `rfind`, `split`, `contains`, `starts_with`, `ends_with` and the
  `trim_*_matches` ([0157](decisions/0157-char-predicates.md)); `get(range)`
  is `None` where slicing panics. `splitn`, `rsplit`, `split_terminator`, `split_at`,
  `match_indices`, `matches` and the `trim_*_matches` search as Rust's do,
  `rsplit` from the end ([0150](decisions/0150-string-patterns.md)).

Differences:

- **`trim()` uses JavaScript's whitespace:** it removes U+FEFF, which Rust
  keeps, and keeps U+0085, which Rust removes.
  `split_whitespace` is exact.
- **Strings compare by UTF-16 units:** `<`, `cmp` and `sort()` of strings
  can order a character above U+FFFF differently than Rust does.

- **A type's own `FromStr` is what `parse` calls,** `s.parse::<Role>()`
  calling `impl FromStr for Role`'s `from_str`
  ([0159](decisions/0159-user-from-str.md)); a generic `T: FromStr`'s is
  its dictionary's, std's or the crate's ([0161](decisions/0161-generic-from-str.md)).

Refused, among others: `str::from_utf8`, `String::from_utf8`,
`make_ascii_uppercase` of a `String`, and a C string, `c"..."`.

## Formatting and printing

`format!` is a template literal; `println!` is `console.log`, `eprintln!`
`console.error`, and `print!` writes without a newline
([0087](decisions/0087-printing.md), [0132](decisions/0132-std-odds.md)).

- **Every placeholder option is Rust's:** width, fill, alignment, sign, `#`,
  `0`, precision, `{:x}`, `{:b}`, `{:o}`, `{:e}`, positional and named
  arguments ([0058](decisions/0058-format-options.md)).
- **Floats print Rust's shortest digits:** `{}` never uses an exponent,
  `{:?}` of `1.0` is `1.0`, and `{:.0}` of `2.5` is `2`
  ([0064](decisions/0064-numbers.md)).
- **`{:?}` is Rust's `Debug`:** a type's name and fields, a string's
  escapes, `{:#?}` pretty ([0060](decisions/0060-debug.md),
  [0137](decisions/0137-pretty-debug.md)).
- **A type's own `Display` and `Debug` are given the placeholder's
  options,** and can ask its `Formatter` for them, `f.width()`, `f.pad(s)`
  ([0143](decisions/0143-formatter-options.md)).
- **`write!` and `writeln!` into a `String` are `s += ..`,** and the
  `fmt::Result` they give is always `Ok`: its `unwrap()` is `()`
  ([0148](decisions/0148-write-to-string.md)).
- **Arguments are taken in Rust's order:** `println!("{} {:?}", v.len(),
  v.pop())` reads the length before the pop
  ([0034](decisions/0034-strings-and-chars.md)).
- **`{:x}`, `{:e}` and `{:p}` of the crate's types call its own `LowerHex`,
  `LowerExp`, `Pointer` and the like,** given the placeholder's options
  ([0165](decisions/0165-other-fmt-traits.md)).
- **A writer of the crate's, `impl fmt::Write`, is given its text whole:**
  one `write_str` for a `write!`, where Rust's gives it a piece at a time
  ([0166](decisions/0166-user-fmt-write.md)).

Refused: `{:x?}`; `{:.2e}`; options for a `&dyn Debug` made elsewhere;
`f.sign_minus()` and `f.pad_integral(..)`; a writer that fails; `{:p}` of
a reference; `LowerHex::fmt(&n, f)` of a number.

## Values, copying and mutation

| Rust | JS |
|---|---|
| struct | object, `{ x, y }` ([0020](decisions/0020-structs-and-tuples.md)) |
| tuple, tuple struct | array, `[a, b]` |
| unit struct, `()` | `undefined` |
| fieldless enum variant | its name, `"Green"` ([0013](decisions/0013-fieldless-enums.md)) |
| enum variant with fields | `{ TAG: "Circle", r }`, `{ TAG: "Some", _0: x }` ([0033](decisions/0033-enums-with-fields.md)) |
| `Result` | `{ TAG: "Ok", _0: v }`, `{ TAG: "Err", _0: e }` ([0035](decisions/0035-results-and-throwing-js.md)) |
| `Option` | the value, or `undefined` for `None` ([0030](decisions/0030-option.md)); a `Some` of what could look like `None`, `Option<()>` or `Option<Option<T>>`, is boxed, `{ $someNone: n }` ([0051](decisions/0051-generic-options.md)) |
| `Box<T>`, `Rc<T>`, `Arc<T>` | the value itself |
| `Cell`, `RefCell`, `Mutex`, `RwLock`, an atomic, `static mut` | `{ value }` ([0025](decisions/0025-vec-loops-refcell-mut.md), [0144](decisions/0144-locks.md)) |
| `Vec`, slice, array | array |

- **A move never copies:** Rust won't let the old name be used again.
- **A `Copy` value is copied only if its type is changed in place
  somewhere,** `{ ...p }`; otherwise sharing the object can't be told apart
  from copying it. A derived `clone()` copies what needs copying, and
  `vec![x; n]` makes `n` separate values
  ([0052](decisions/0052-std-trait-impls.md),
  [`copy_mutation`](../test/corpus/copy_mutation.rs)).
- **`&T` is the value. `&mut` to an object is the object**, changed in place
  ([0025](decisions/0025-vec-loops-refcell-mut.md)). A `&mut` to a number, a
  string or an `Option` names its place when it's in a variable, `*r += 1`
  being `x += 1`; given to a function, it's a box `{ value }` the caller
  reads back; kept elsewhere, a handle with a getter and a setter
  ([0074](decisions/0074-mut-boxes.md),
  [0099](decisions/0099-mut-references.md)).
- **`*r = v` replaces the value whole, as Rust's does:** an object `r` is
  becomes `v` in place, `$assign(r, v)`, so every name for it sees `v`;
  `mem::replace`, `mem::swap` and `mem::take` the same. An enum with a
  fieldless variant, whose `Off` is a string, is reached through its place
  instead, as a number is ([0147](decisions/0147-replacing-through-mut.md)).
- **`Rc::clone` shares the one object,** as Rust's does.
- **A `&mut` std gives to a number or a string in a collection is a
  handle on it,** where it's used as a value: `iter_mut()`, `values_mut()`,
  `get_mut`, `last_mut()` and a `for` over `&mut m`, each writing the slot
  it's of ([0152](decisions/0152-std-item-handles.md)).

Differences:

- **`RefCell` borrows aren't checked:** a second `borrow_mut()` while one is
  alive doesn't panic ([0025](decisions/0025-vec-loops-refcell-mut.md)).
- **Locks aren't checked:** a `Mutex` locked twice on one thread doesn't
  hang, and a lock is never poisoned ([0144](decisions/0144-locks.md)).

Refused: `Rc::ptr_eq`, `strong_count`, `get_mut`, `make_mut` and
`try_unwrap`; `Weak`; `Cell::swap`; `RefCell::try_borrow_mut`; `OnceCell`,
`OnceLock` and `LazyLock`; `Mutex::try_lock`; a map's `get_mut` bound by a
`match` and kept past it.

## Collections and iterators

| Rust | JS |
|---|---|
| `Vec`, `VecDeque` | array ([0068](decisions/0068-queues.md)) |
| `BinaryHeap` | array, in Rust's heap order: ties pop as Rust's do |
| `HashMap`, `HashSet` | `Map`, `Set` ([0059](decisions/0059-hashmap.md)); keyed by value for a struct, tuple or enum key with a derived `Eq`, `$KeyMap`, whatever its `Hash` ([0121](decisions/0121-value-keys.md), [0168](decisions/0168-user-hash.md)) |
| `BTreeMap`, `BTreeSet` | `Map`, `Set`, iterated in key order |

- **An iterator chain is an array's methods,** `v.map(f).filter(p)`, when
  its closures do nothing anyone can see. When one prints, changes
  something or may panic, and something after it could tell, the chain is a
  lazy JS iterator, run one item at a time in Rust's order
  ([0036](decisions/0036-iterators-and-sorting.md),
  [0139](decisions/0139-lazy-chains.md)).
- **A shared slice is a copy,** `&v[a..b]`, `split_at` and `get(a..b)`:
  nothing changes `v` while it's borrowed
  ([0153](decisions/0153-collection-and-cell-methods.md)).
- **An `impl Iterator` is the iterator it stands for**
  ([0061](decisions/0061-generic-iterators.md)).
- **A collection of the crate's is its own impls':** `for` over it calls
  its `IntoIterator`, `collect()` its `FromIterator`, `extend` its
  `Extend`, `sum()` its `Sum` ([0160](decisions/0160-user-collections.md)).
  Given where any `IntoIterator` goes, to `v.extend(cart)` or generic code,
  it's refused: that code wouldn't call its `into_iter`. An iterator of the
  crate's runs from both ends by its own `next_back`, `rev()` included, and
  its `len()` is its `size_hint()`, checked ([0164](decisions/0164-double-ended-iterators.md)).
- **A function passed where a closure goes is the arrow that calls it,**
  `.map(str::len)` being `.map((s) => $byteLen(s))` and `fold(0, i32::max)`
  being `reduce((a, b) => Math.max(a, b), 0)`; a constructor's makes what
  its call makes ([0125](decisions/0125-function-values.md),
  [0151](decisions/0151-called-function-values.md)). One taking a `&mut`
  is refused.

Differences:

- **A `HashMap` or a `HashSet` iterates in insertion order,** where Rust's
  order is unspecified; serde_json writes one in that order too.
- **A chain a function returns or passes on is an array:** each of its
  closures runs over every item, though the caller takes only the first.

Refused: a `BTreeMap` keyed by a struct; a map key with a custom
`PartialEq`; looking a key up, or joining items, by what the crate's own
`Borrow` gives; `==` of two maps; `peekable` or `rev` of a lazy iterator.

## Destructors

`Drop` runs where Rust runs it: in reverse order of declaration, not for a
value moved away, on reassignment, at the end of a temporary, on an early
`return`, and while a panic unwinds, each through a `try`/`finally`. Code
with no `Drop` impl has no extra JS ([0098](decisions/0098-destructors.md)).

Differences: a value with a destructor that JS holds, given out by an
exported generic function, is never dropped; a `drop` that panics while
another panic unwinds replaces that panic, where Rust aborts.

Refused: an `Rc` or an `Arc` of a value with a destructor; a lock or
`async` code owning one.

## Panics and errors

A panic is `throw new Error(message)`, with Rust's message: `panic!`,
`unwrap()` of `None` or of an `Err` (with the error's own `{:?}`),
`expect`, an index out of bounds, an `assert_eq!` with both sides, a map's
missing key, `unreachable!`, `todo!` ([0012](decisions/0012-panics-and-runtime-helpers.md),
[0026](decisions/0026-testing.md)).

- **`Result` and `?` are Rust's:** `?` returns early, through the crate's
  own `From` impls; `Box<dyn Error>` takes the crate's errors, strings,
  `"msg".into()`, std's parse errors and serde_json's
  ([0035](decisions/0035-results-and-throwing-js.md),
  [0141](decisions/0141-std-trait-objects.md)).
- **Control flow is Rust's:** `loop` with a value, labeled `break` and
  `continue`, labeled blocks ([0158](decisions/0158-labeled-blocks.md)),
  `let`-`else`, `if let` chains, match guards, `let` guards.

Differences:

- **A crate is a library: nothing calls `main`.** A host calls it, and an
  uncaught panic is the host's error: a JS stack trace and exit code 1,
  where Rust prints `thread 'main' panicked at` and exits with 101. A
  `main` that returns `Err` returns it to its caller.
- **JS can catch a panic,** an ordinary `Error`; Rust code can't
  (`catch_unwind` is refused).

Refused: `catch_unwind`, `panic::set_hook`, `process::exit`,
`process::abort`; `pin!`, which is `Pin::new_unchecked`.

## Async, threads and time

`async fn` is an `async function`, `.await` is `await`, and an `async move`
block or an `async` closure is an async arrow
([0029](decisions/0029-async-await.md)). A JS caller of one gets a
`Promise`.

- **A future starts when it's made,** runs to its first `await` at once,
  and runs to its end though nothing awaits it, as a JS promise does.
  Rust's waits until it's polled. This is what lets JS call an `async fn`
  and get an ordinary promise, with no Rust executor.
- **`js::spawn` starts a future no one awaits;** `js::settle(p).await`
  makes a rejected promise an `Err` ([0102](decisions/0102-js-and-webapi.md)).
- **There's one thread.** `std::thread::spawn` is refused; a channel is a
  queue its ends share, and `recv()` of an empty one whose senders are
  alive panics, where Rust's would wait forever
  ([0142](decisions/0142-channels.md)).

Refused: `Box::pin`, so a recursive `async fn`; `future::ready`, `poll_fn`;
a user `IntoFuture`; joining futures, `Promise.all`; `Duration`, `Instant`,
`SystemTime` and `thread::sleep`.

## Tests

`rust-js --test` writes a crate's `#[test]` functions and a `.test.js`
that registers each, for `bun test` or a browser runner
([0026](decisions/0026-testing.md), [0027](decisions/0027-real-browser-tests.md)).
`#[should_panic]` and `#[ignore]` are libtest's, assertion messages are
Rust's, and a test returning a `Result` fails on `Err`. A struct error in
that failure shows without its type's name.

## The JavaScript boundary

**A crate is a library of ES modules,** one per Rust module, `mod shapes`
being `shapes.js`, or `.jsx` with JSX in it
([0019](decisions/0019-one-js-file-per-module.md),
[0073](decisions/0073-named-module-imports.md)). Each imports the runtime
helpers it uses from `@rust-js/runtime`, which must be the compiler's
version ([0103](decisions/0103-runtime-package.md)).

What JS sees of a crate:

- **A `pub fn` is an `export function` of its Rust name**; parameters and
  locals are camelCase, and a name JS reserves gets a `$`,
  `delete$`. With `js::camel_case!();`, the crate's functions, fields and
  props are camelCase too ([0038](decisions/0038-js-names-and-destructuring.md),
  [0046](decisions/0046-camel-case-crates.md)).
- **A type's methods are its exported object's,** the receiver first:
  `Point.shift(p, by)` ([0047](decisions/0047-methods.md)).
- **`FromStr`, `AsRef` and `Borrow` are dictionaries too:** a generic
  `S: AsRef<str>` is given `{ as_ref }`, a `K: Borrow<Q>` `{ borrow }`,
  the value itself for a `String` or a `&str`
  ([0161](decisions/0161-generic-from-str.md),
  [0162](decisions/0162-generic-as-ref.md), [0167](decisions/0167-borrow.md)).
- **A generic function takes its trait dictionaries after its arguments,**
  `show(t, pointDisplay())`; a `dyn Trait` is `{ value, impl }`
  ([0049](decisions/0049-traits-and-generics.md)).
- **Values are the shapes above:** a struct an object literal, a `u64` a
  BigInt, `None` `undefined` or `null`, a `&mut` to a number `{ value }`, a
  closure a function, an `async fn`'s result a `Promise`.
- **`pub use` doesn't re-export:** JS imports an item from the module that
  defines it.
- **A crate's JS has no stable API for hand-written JS yet:** its shape is
  the compiler's that wrote it ([0100](decisions/0100-separate-crates.md)).

Calling JavaScript:

- **A binding is a Rust function with a `link_name`:** a global, a method
  (`this` first), a property, `new X`, an import from a module
  ([0021](decisions/0021-js-interop.md), [0028](decisions/0028-js-module-imports.md),
  [0039](decisions/0039-generic-bindings.md)). `js::import!("./app.css")`
  imports a file for its effects ([0110](decisions/0110-stable-syntax.md)).
- **The `js` crate binds JS's own globals, the `webapi` crate the
  browser's,** generated from its WebIDL; options objects are structs of
  `Option`s ([0102](decisions/0102-js-and-webapi.md)). A library's binding
  is an npm package ([0118](decisions/0118-bindings-on-npm-only.md)).
- **A binding is trusted as declared.** One returning
  `Result<T, &JsError>` makes what JS throws an `Err`; any other lets it
  propagate, as a panic. Declare `Option` where JS may give `null`.

JSON through serde_json writes and reads as serde_json does, a 64-bit
integer to the digit; `rename`, `tag`, `untagged`, `flatten`, `default`,
`skip` and the other common attributes are supported
([0077](decisions/0077-serde-json.md)–[0083](decisions/0083-serde-json-value.md)).
Refused: `#[serde(with)]`, `serialize_with`, `deserialize_with`, a struct as
a map key. React and JSX are [their own page](jsx.md).

## Differences from native Rust, in one list

1. Integer overflow wraps, as a release build's does, in every build.
2. `usize` and `isize` are 32 bits.
3. `i64` and `u64` are BigInts at the JS boundary.
4. Float library functions, `sin`, `exp`, `powf`, are JS's: the last bit may
   differ. A NaN's sign and payload aren't kept.
5. `trim()` uses JS's whitespace; strings compare by UTF-16 units.
6. A `HashMap` iterates in insertion order.
7. A chain a function returns runs its closures over every item.
8. `RefCell` borrows and locks aren't checked.
9. A future starts when it's made, and runs though no one awaits it.
10. There's one thread: a channel's `recv()` that would wait forever panics.
11. Nothing calls `main`; an uncaught panic is the host's error, exit code 1.
12. JS can catch a panic; a `drop` panicking during a panic replaces it.
13. A value with a destructor that JS holds is never dropped.
14. A writer of the crate's is given each `write!`'s text in one `write_str`.
