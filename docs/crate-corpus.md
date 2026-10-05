# Crates from crates.io, compiled to JS

What a shared model's dependencies come to under rust-js (ROADMAP M8.2).
`bun scripts/crate-corpus.ts` compiles each crate below in a Cargo project
of its own, every library of its graph by rust-js as Cargo's
`RUSTC_WRAPPER`, for its target, and records what each refused first, or
what Cargo said where a crate failed with no refusal: a crash.

Compiled isn't run. Where a crate compiles, its probe,
[`scripts/crate-corpus/<name>.rs`](../scripts/crate-corpus/), a library that
uses it as a shared model would, is compiled with it, and its `report()` run,
the JS's beside native Rust's: **runs** where they're the same. A derive's
code is the probe's: strum's and thiserror's compiled before what they write
in a crate that uses them did (ADR 0186).

Measured 2026-10-05, after ADR 0189, Rust 1.98.1, each crate at the
newest release its requirement allows.

| Crate | Verdict | First refusals in its graph |
|---|---|---|
| strum 0.27 (`derive`) | runs | |
| either 1.18 | refused | a user `Future` |
| itertools 0.14 | blocked | either's |
| indexmap 2.11 | blocked | hashbrown: a raw pointer |
| bitflags 2.13 | runs | |
| smallvec 1.16 | refused | `handle_alloc_error` called |
| thiserror 2.0 | runs | |
| anyhow 1.0 | refused | a raw pointer |
| once_cell 1.21 | refused | a constant of a raw pointer |
| semver 1.0 | refused | a constant of a `NonNull` |
| uuid 1.27 | refused | `&mut` of a `MaybeUninit` buffer's range |
| url 2.5 | blocked | litemap: a loop over items with a destructor; writeable: `u8::checked_ilog10` called; smallvec's; percent-encoding: `transmute`; yoke: a raw pointer |
| regex 1.11 | blocked | memchr: a raw pointer |
| rust_decimal 1.38 | blocked | arrayvec: a user `io::Write`; serde_core: `size_of` of a type parameter |
| chrono 0.4 (`alloc`) | refused | a loop over items with a destructor |
| time 0.3 (`alloc`) | blocked | powerfmt: a `MaybeUninit`; deranged: a `const` block |

3 of 16 compile, and each runs as natively. What stops the most, by the crates it stops:

1. **Raw memory,** a raw pointer, a `NonNull`, a `MaybeUninit`,
   `transmute`, `handle_alloc_error`: 9.
2. **A user `Future`:** 2. **A loop over items with a destructor:** 2,
   litemap and chrono.
3. **One each:** a `const` block, a user `io::Write`, `size_of` of a type
   parameter, `u8::checked_ilog10`.

## Fixed by measuring

- **std's `Duration`, `panic!("{}", x)`, a slice into an array and
  `into_iter()` of the crate's iterator:** chrono's 18 refusals are 9 (ADRs
  0188, 0189).
- **A writer's `Err(fmt::Error)`,** chrono's, at four places: thrown, with
  what was written before it, each consumer taking it as std's does (ADR
  0187).
- **What another crate's derive writes,** strum's `IntoEnumIterator` and
  thiserror's `From`, was skipped as std's derives' is; `Display` of a
  `fmt::Arguments`; std's `Error` of a parse error as a dictionary: strum
  and thiserror run (ADR 0186).
- **`ok_or_else` of a value with a destructor, a temporary of one in a
  branch, and `by_ref()` of an iterator of the crate's:** bitflags compiles
  (ADR 0184). **A library's trait's defaults, a number's `LowerHex` given a
  `Formatter`, and modules of one name in blocks:** a crate that uses its
  macro compiles, and runs as natively (ADR 0185).
- **A clone of std's iterator over an array,** chrono's `DelayedFormat`
  items (ADR 0181).
- **Generic code that writes to any `fmt::Write`,** chrono's, 7 refusals,
  and bitflags' `to_writer`, given a `Formatter` (ADR 0180).
- **A byte's ASCII tests and a slice's `split_first`,** chrono's (ADRs 0157,
  0153).
- **`Option::filter` and `map_or` of a value with a destructor,** chrono's
  (ADR 0179).
- **A value of an associated type where a type may have a destructor,**
  chrono's `Tz::Offset`, 55 refusals (ADR 0178).
- **A `NonZero` integer,** a pattern type `(u8) is 1..` in std, stopped
  chrono, at 333 places, and deranged (ADR 0177).
- **num-traits compiles,** after `Wrapping`, a float's methods, every
  integer's bits, defaults of `Self`, and a generic impl's constant of its
  parameters (ADRs 0156, 0175, 0176).
- **`{:p}` of a generic `T`:** thiserror compiles now (ADR 0174).
  **`f32::is_normal` and `classify`,** num-traits' (ADR 0122); **the bit
  methods of every integer and a float's bytes,** num-traits' `PrimInt` and
  `Float`, 189 refusals (ADR 0156). **`std::num::Wrapping`,** num-traits'
  (ADR 0175).
- **A path's `display()`,** thiserror's (ADR 0173); **`Option::map` of a
  value with a destructor and a generic `Option<T>`'s drop,** num-traits' and
  zerofrom's, which compiles now (ADR 0098); **`ParseIntError::kind()` and
  `{:?}` of `IntErrorKind`,** deranged's (ADR 0063).
- **Bytes to text,** `str::from_utf8`, `from_utf8_lossy` and the like:
  regex-syntax compiles now (ADR 0172). **`f32::to_int_unchecked`,**
  num-traits' (ADR 0171).
- **`u128` and `i128`** stopped num-traits, num-conv, zerofrom and
  serde_core; num-conv and time-core compile now (ADR 0171).
- **A `#![no_std]` crate's std items,** which rustc names `core::str::Chars`,
  weren't recognized: num-traits' `chars()`. **`size_hint()` of a generic
  iterator,** serde_core's, bounds that hold, a listed difference (ADR 0170).
- **`?` of a value with a destructor,** num-traits' `checked_pow` (ADR
  0098); **`char::from_u32_unchecked`,** utf8_iter's, which now compiles
  (ADR 0157); **`split_at_checked`,** writeable's (ADR 0153).
- **`size_hint()`** of an iterator of the crate's, utf8_iter's (ADR 0170);
  **a slice's `starts_with` and `eq_ignore_ascii_case` of bytes,**
  writeable's and uuid's (ADR 0153).
- **Two traits of one name for one type,** serde_core's `de::Error` and
  `ser::Error`, named alike (ADR 0133); **a user `AsMut` or `BorrowMut`,**
  smallvec's and arrayvec's (ADR 0169); **a slice's `split_at` and `get`
  of a range,** uuid's and writeable's (ADR 0153).
- **A user `Hash`** stopped semver, uuid, arrayvec and powerfmt (ADR 0168).
- **A user `Borrow`, and `Borrow::borrow` of a generic key,** stopped uuid,
  deranged and equivalent (ADR 0167).
- **A user `fmt::Write`** stopped anyhow, writeable and serde_core
  (ADR 0166).
- **A user `LowerHex`, `Pointer` and the like** stopped uuid and thiserror
  (ADR 0165).
- **A user `DoubleEndedIterator` or `ExactSizeIterator`** stopped 8
  (ADR 0164).
- **A generic trait method where a type may have a destructor** stopped
  9 (ADR 0163).
- **num-traits' supertrait dictionary names colliding** was an internal
  error (ADR 0106).
- **A `#![no_std]` crate's trait call crashed rust-js,** which looked up
  `alloc`'s `ToString` there: bitflags', num-traits'.
