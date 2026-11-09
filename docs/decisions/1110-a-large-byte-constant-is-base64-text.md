# 1110. A large byte constant is base64 text, decoded into its array

Status: Accepted (#1096; the user's choice, 2026-11-09). Extends
[0126](0126-byte-strings.md), whose byte strings are arrays of numbers.

Case ([0262](0262-when-rust-and-js-disagree.md)): N. It's how the JS reads
and how the compiler holds it; each reader sees the array it saw.

## Context

A `&[u8; N]` constant was a literal of each byte, `[71, 69, 84]`, which
rustc's value tree made first, a node a byte. windows-bindgen's
`expand_input` `include_bytes!`s 32 MB of Windows metadata: its lowering
took rust-js 7–9 GB and about a minute, each step of the body's lowering
walking the array again (`visit_vars`, `reads_only_vars`), and wrote a
literal no one reads. Measured by size, one `include_bytes!` in a program:

| Bytes | Literal: time, memory, JS | base64 |
|---|---|---|
| 1 KiB | 0.06 s, 112 MB, 5 KB | |
| 16 KiB | 0.09 s, 120 MB, 78 KB | |
| 256 KiB | 0.82 s, 485 MB, 1.2 MB | 0.06 s, 114 MB, 350 KB |
| 1 MiB | 3.1 s, 1.65 GB, 5 MB | 0.09 s, 119 MB, 1.4 MB |

## Decision

- **A byte constant of more than 16 KiB, `LARGE_BYTES`, is
  `$bytes("..")` of its base64 text**: read from its memory, never a value
  tree of each byte, by `representation::byte_constant_js`, the one place
  the form is written, of a constant a body writes in place,
  `include_bytes!`'s, a `const { .. }` block's or a promoted one's, and of
  a `const` item's, `&[u8; N]` or `&[u8]`. To 16 KiB a literal costs the
  compiler nothing past its own and reads as the bytes it is; past it the
  literal's cost grows by about 1.5 KB of memory and 4.8 bytes of JS a
  byte, base64's by 1.33 bytes.
- **`$bytes` makes the array of numbers the literal is**, each time it's
  read, as the literal is made each time: every reader, indexing, `iter()`,
  slicing, `to_vec()`, `==`, generic code and `{:?}`, sees the form it
  saw, and no reader shares an array another may change.
- **A constant's array of literals is known so**, `Known::Literal`, which
  the walks that ask what an expression reads or does don't enter: one
  of numbers past the limit, a `u32` table's, is cheap to lower too.

## Consequences

- windows-bindgen's crate: 4 s and 0.48 GB, from about 55 s and 9 GB.
- `test/corpus/byte_constants_large.rs` holds both forms, at the limit and
  past it, named, in place and a slice, each read every way, against native
  Rust; the mutations `large-bytes-at-the-limit`, `large-bytes-unpadded`,
  `large-bytes-named-as-literal`, `large-bytes-slice-in-memory-unread`,
  `large-bytes-in-place-as-literal`, `literal-array-unknown` and
  `literal-array-of-any` put each part back, the last two held by the
  unit test `a_constants_array_of_literals_is_known_literal`.
- A `static`'s bytes are its memory's value tree still: none past the limit
  has been seen.
