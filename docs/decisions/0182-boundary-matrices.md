# 0182. Boundary matrices: each operation on the values where implementations go wrong

Status: Accepted. Extends [0093](0093-mutations.md) and the corpus's
comparison with native Rust.

Case: N ([0262](0262-when-rust-and-js-disagree.md)).

## Context

The corpus's cases are programs someone thought of; the generated programs
(ADR 0092) are random. Neither tries each operation rust-js supports on
each value where a JS implementation goes wrong: a type's `MIN` and
`MAX`, `-0.0`, a NaN, a character past U+FFFF, U+0085 and U+FEFF. Today's
fixes were found that way, one at a time: `NonZero` of 0, a byte's ASCII
tests, `u8` wrapping.

topcoat's coherence harness compares each expression of its Rust subset
with its JS on matrices of such values, a float by its bits. Its first
lesson for rust-js is the matrix; its harness, an embedded V8 comparing
values directly, isn't needed: the corpus already compares a program's
output with native Rust's, under bun, Node and a minified build.

## Decision

**`scripts/matrix.ts` writes the boundary matrices, `test/corpus/matrix_*.rs`:
corpus cases that try each supported operation of integers, floats,
strings and chars on each value, and each pair of values, where
implementations go wrong, and print each result exactly:**

| Matrix | Values | Each |
|---|---|---|
| `matrix_integers` | `MIN`, `MIN + 1`, `-1`, `0`, `1`, `2`, `7`, `MAX - 1`, `MAX`, of each width to 128 bits | wrapping, checked, saturating and overflowing arithmetic, shifts past the width, bit counts, formats, casts |
| `matrix_floats` | `±0.0`, halves, `0.1`, a subnormal, `MIN_POSITIVE`, `EPSILON`, `MAX`, `±∞`, NaN, `2^53 + 1`, of `f64` and `f32` | arithmetic, rounding, `powf` of a whole or infinite power, formats, casts, orderings |
| `matrix_strings` | ASCII, combining marks, `ß`, `İ`, U+0085, U+FEFF, U+2028, NUL, a character past U+FFFF | each method, and each comparison of each pair, by `{:?}` |
| `matrix_chars` | the same, and digits, Greek sigma, `ﬁ` | each predicate, case map, digit, cast and comparison |

- **A float is printed by its bits,** as `{}` can hide a last bit; a NaN as
  `NaN`, as its bits differ by platform.
- **What's left out is listed, with why,** in the generator's `excluded`:
  what rust-js refuses yet, which would refuse the whole case, and what
  differs by design: `usize` (32 bits here, 64 natively), a float library
  function's last bit, a fractional power's.
- **A listed difference is shown, not skipped:** where strings' order by
  UTF-16 units, or JS's `trim()`, would differ, as the semantics page
  lists, the line says `listed`, found the same way on both sides.
  (Amended: ADR 0183 closed both; the matrices print them whole.)
- **The generator writes them, and a test checks it did:** an edit goes in
  `scripts/matrix.ts`, `bun run matrix`.

## Why

- **It found what nothing else had:** a negative `i128`'s `{:x}`, signed;
  `1.0.powf(f64::INFINITY)` and `(-1.0).powf(f64::INFINITY)`, NaN as
  JS's `**` has them, where Rust's are 1; `String::from_utf8_lossy` and
  serde_json dropping a U+FEFF at a string's start, as `TextDecoder` drops
  a byte order mark. Each is fixed, with a mutation its matrix catches.
- **It's cheap:** four cases, about 5,000 lines of output, a second each.

## Alternatives

- **topcoat's harness:** each expression's value compared directly, in an
  embedded engine. Exact to the value, but a second harness beside the
  corpus, whose comparison with native Rust already covers this, by
  printing exactly.
- **Random values,** as ADR 0092's generated programs: they find what no
  one listed, but rarely a type's `MIN` paired with `-1`. The matrices
  are the values that matter, every time.

## Costs

- **A new std operation goes in its matrix too,** and one that's refused
  is listed out until it's supported.
- **The integer matrix's JS snapshot is large,** 57 KB, its macro
  expanded once for each type.
