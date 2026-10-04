# Crates from crates.io, compiled to JS

What a shared model's dependencies come to under rust-js (ROADMAP M8.2).
`bun scripts/crate-corpus.ts` compiles each crate below in a Cargo project
of its own, every library of its graph by rust-js as Cargo's
`RUSTC_WRAPPER`, for its target, and records what each refused first, or
what Cargo said where a crate failed with no refusal: a crash.

Measured 2026-10-04, after ADR 0165, Rust 1.98.1, each crate at the
newest release its requirement allows.

| Crate | Verdict | First refusals in its graph |
|---|---|---|
| strum 0.27 (`derive`) | compiles | |
| either 1.18 | refused | a user `Future` |
| itertools 0.14 | blocked | either's |
| indexmap 2.11 | blocked | equivalent: `Borrow::borrow`; hashbrown: a raw pointer |
| bitflags 2.13 | refused | `fmt::Write::write_str` called |
| smallvec 1.16 | refused | a user `AsMut` |
| thiserror 2.0 | refused | `Path::display` called |
| anyhow 1.0 | refused | a user `fmt::Write` |
| once_cell 1.21 | refused | a constant of a raw pointer |
| semver 1.0 | refused | a user `Hash` |
| uuid 1.27 | refused | a user `Borrow` |
| url 2.5 | blocked | utf8_iter: `size_hint` called; litemap: `{:?}` of a `PhantomData`; writeable: a user `fmt::Write`; smallvec's; percent-encoding: `transmute`; zerofrom: `u128` |
| regex 1.11 | blocked | memchr: a raw pointer; regex-syntax: `str::from_utf8` |
| rust_decimal 1.38 | blocked | arrayvec: a user `Hash`; serde_core: a user `fmt::Write`; num-traits: a value with a destructor bound where it isn't supported |
| chrono 0.4 (`alloc`) | blocked | num-traits' |
| time 0.3 (`alloc`) | blocked | powerfmt: a user `Hash`; deranged: a user `Borrow`; num-conv: `u128`; time-core: a generic impl's constant of its parameters |

1 of 16 compiles. What stops the most, by the crates it stops:

1. **A user `fmt::Write`,** or `write_str` of a generic writer: 4.
2. **Raw memory,** a raw pointer, `transmute`: 4.
3. **A user `Hash`:** 3. **A user `Borrow`, and `Borrow::borrow`:** 3.
4. **`u128`:** 2. **num-traits' value with a destructor bound:** 2.
   **A user `Future`:** 2. **A user `AsMut`:** 2.
5. **One each:** `size_hint` called, `{:?}` of a `PhantomData`, `Path::display`,
   `str::from_utf8`, a generic impl's constant.

## Fixed by measuring

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
