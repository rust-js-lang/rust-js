# Crates from crates.io, compiled to JS

What a shared model's dependencies come to under rust-js (ROADMAP M8.2).
`bun scripts/crate-corpus.ts` compiles each crate below in a Cargo project
of its own, every library of its graph by rust-js as Cargo's
`RUSTC_WRAPPER`, for its target, and records what each refused first, or
what Cargo said where a crate failed with no refusal: a crash.

Measured 2026-10-04, after ADR 0172, Rust 1.98.1, each crate at the
newest release its requirement allows.

| Crate | Verdict | First refusals in its graph |
|---|---|---|
| strum 0.27 (`derive`) | compiles | |
| either 1.18 | refused | a user `Future` |
| itertools 0.14 | blocked | either's |
| indexmap 2.11 | blocked | hashbrown: a raw pointer |
| bitflags 2.13 | refused | `fmt::Write::write_str` of a generic writer called |
| smallvec 1.16 | refused | `handle_alloc_error` called |
| thiserror 2.0 | refused | `Path::display` called |
| anyhow 1.0 | refused | a raw pointer |
| once_cell 1.21 | refused | a constant of a raw pointer |
| semver 1.0 | refused | a constant of a `NonNull` |
| uuid 1.27 | refused | `&mut` of a `MaybeUninit` buffer's range |
| url 2.5 | blocked | litemap: `{:?}` of a `PhantomData`; writeable: a whole value assigned through a `&mut`; smallvec's; percent-encoding: `transmute`; zerofrom: dropping an `Option<T>` |
| regex 1.11 | blocked | memchr: a raw pointer |
| rust_decimal 1.38 | blocked | arrayvec: a user `io::Write`; serde_core: `size_of` of a type parameter; num-traits: `Option::map` of a value with a destructor |
| chrono 0.4 (`alloc`) | blocked | num-traits' |
| time 0.3 (`alloc`) | blocked | powerfmt: a `MaybeUninit`; deranged: `{:?}` of an `IntErrorKind` |

1 of 16 compiles. What stops the most, by the crates it stops:

1. **Raw memory,** a raw pointer, a `NonNull`, a `MaybeUninit`,
   `transmute`, `handle_alloc_error`: 9.
2. **num-traits' `Option::map` of a value with a destructor:** 2, chrono and
   rust_decimal.
   **A user `Future`:** 2.
3. **One each:** `{:?}` of a `PhantomData`, `Path::display`, `write_str` of a
   generic writer, `{:?}` of an `IntErrorKind`, a user `io::Write`, `size_of`
   of a type parameter, a whole value assigned through a `&mut`, dropping
   an `Option<T>`.

## Fixed by measuring

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
