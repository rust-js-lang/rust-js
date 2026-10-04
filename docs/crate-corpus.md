# Crates from crates.io, compiled to JS

What a shared model's dependencies come to under rust-js (ROADMAP M8.2).
`bun scripts/crate-corpus.ts` compiles each crate below in a Cargo project
of its own, every library of its graph by rust-js as Cargo's
`RUSTC_WRAPPER`, for its target, and records what each refused first.

Measured 2026-10-04, rust-js `b9e6ef6`, Rust 1.98.1, each crate at the
newest release its requirement allows.

| Crate | Verdict | First refusals in its graph |
|---|---|---|
| strum 0.27 (`derive`) | compiles | |
| either 1.18 | refused | a user `DoubleEndedIterator` |
| itertools 0.14 | blocked | either's |
| indexmap 2.11 | blocked | equivalent: `Borrow::borrow`; hashbrown: a generic trait method where a type may have a destructor |
| bitflags 2.13 | refused | a generic trait method given a value with a destructor |
| smallvec 1.16 | refused | a generic trait method where a type may have a destructor |
| thiserror 2.0 | refused | a user `fmt::Pointer` |
| anyhow 1.0 | refused | a generic trait method where a type may have a destructor |
| once_cell 1.21 | refused | a constant of a raw pointer |
| semver 1.0 | refused | a generic trait method where a type may have a destructor |
| uuid 1.27 | refused | a user `fmt::LowerHex` |
| url 2.5 | blocked | utf8_iter: a user `DoubleEndedIterator`; percent-encoding: `transmute`; litemap: a user `ExactSizeIterator`; writeable: a user `fmt::Write`; smallvec's; zerofrom: `u128` |
| regex 1.11 | blocked | memchr: a user `DoubleEndedIterator`; regex-syntax: a generic trait method where a type may have a destructor |
| rust_decimal 1.38 | blocked | arrayvec: a generic trait method where a type may have a destructor; num-traits: a generic trait method given a value with a destructor; serde_core: a user `fmt::Write` |
| chrono 0.4 (`alloc`) | blocked | num-traits: a generic trait method given a value with a destructor |
| time 0.3 (`alloc`) | blocked | powerfmt: a user `Hash`; deranged: a user `Borrow` |

1 of 16 compiles. What stops the most:

1. **A generic trait method where a type may have a destructor:** 9 crates.
2. **A user `DoubleEndedIterator` or `ExactSizeIterator`:** 4.
3. **A user `fmt` trait other than `Display` and `Debug`,** `Pointer`,
   `LowerHex`, `fmt::Write`: 4.
4. **A user `Hash` or `Borrow`, and `Borrow::borrow`:** 2.
5. **Raw memory:** `transmute`, `u128`, a raw pointer constant: 3.

num-traits stopped first at an internal error, its supertrait dictionary
names colliding, which is fixed (ADR 0106); it stops at the first gap now.
