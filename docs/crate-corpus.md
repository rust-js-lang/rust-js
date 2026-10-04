# Crates from crates.io, compiled to JS

What a shared model's dependencies come to under rust-js (ROADMAP M8.2).
`bun scripts/crate-corpus.ts` compiles each crate below in a Cargo project
of its own, every library of its graph by rust-js as Cargo's
`RUSTC_WRAPPER`, for its target, and records what each refused first, or
what Cargo said where a crate failed with no refusal: a crash.

Measured 2026-10-04, after `65f2760`, Rust 1.98.1, each crate at the
newest release its requirement allows.

| Crate | Verdict | First refusals in its graph |
|---|---|---|
| strum 0.27 (`derive`) | compiles | |
| either 1.18 | refused | a user `DoubleEndedIterator` |
| itertools 0.14 | blocked | either's |
| indexmap 2.11 | blocked | equivalent: `Borrow::borrow`; hashbrown: a user `ExactSizeIterator` |
| bitflags 2.13 | refused | `fmt::Write::write_str` called |
| smallvec 1.16 | refused | a user `DoubleEndedIterator` |
| thiserror 2.0 | refused | a user `fmt::Pointer` |
| anyhow 1.0 | refused | a user `DoubleEndedIterator` |
| once_cell 1.21 | refused | a constant of a raw pointer |
| semver 1.0 | refused | a user `Hash` |
| uuid 1.27 | refused | a user `fmt::LowerHex` |
| url 2.5 | blocked | utf8_iter: a user `DoubleEndedIterator`; litemap: a user `ExactSizeIterator`; writeable: a user `fmt::Write`; smallvec's; percent-encoding: `transmute`; zerofrom: `u128` |
| regex 1.11 | blocked | memchr: a user `DoubleEndedIterator`; regex-syntax: `str::from_utf8` |
| rust_decimal 1.38 | blocked | arrayvec: a user `DoubleEndedIterator`; serde_core: a user `fmt::Write`; num-traits: a value with a destructor bound where it isn't supported |
| chrono 0.4 (`alloc`) | blocked | num-traits' |
| time 0.3 (`alloc`) | blocked | powerfmt: a user `Hash`; deranged: a user `Borrow`; num-conv: `u128`; time-core: a generic impl's constant of its parameters |

1 of 16 compiles. What stops the most, by the crates it stops:

1. **A user `DoubleEndedIterator` or `ExactSizeIterator`:** 8.
2. **A user `fmt` trait other than `Display` and `Debug`,** `Pointer`,
   `LowerHex`, `fmt::Write`, or `write_str` called: 5.
3. **A user `Hash` or `Borrow`, and `Borrow::borrow`:** 3.
4. **`u128`:** 2. **num-traits' value with a destructor bound:** 2.
5. **Raw memory and the rest,** one each: `transmute`, a raw pointer
   constant, `str::from_utf8`, a generic impl's constant.

## Fixed by measuring

- **A generic trait method where a type may have a destructor** stopped
  9 (ADR 0163).
- **num-traits' supertrait dictionary names colliding** was an internal
  error (ADR 0106).
- **A `#![no_std]` crate's trait call crashed rust-js,** which looked up
  `alloc`'s `ToString` there: bitflags', num-traits'.
