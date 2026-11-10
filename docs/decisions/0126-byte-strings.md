# 0126. A byte string is its bytes, and `as_bytes()` a string's UTF-8 bytes

Status: Accepted. Extends [0063](0063-text.md).

Case: C, A, D ([0262](0262-when-rust-and-js-disagree.md)).

## Context

A byte string, `b"GET"`, was an error as a value, though one as a pattern
was its bytes already; and so was `s.as_bytes()`. A byte string is a `&[u8;
N]`, which rust-js has as an array of numbers (ADR 0036), and a `str` is a
JS string, of UTF-16 units, not of the UTF-8 bytes Rust's is (ADR 0063).

## Decision

| Rust | JS |
|---|---|
| `b"GET"` | `[71, 69, 84]` |
| `b"a\xff"` | `[97, 255]`: every byte, not only ASCII |
| `s.as_bytes()` | `Array.from(new TextEncoder().encode(s))` |
| `s.bytes()` | the same array, an iterator as any array is (ADR 0036) |
| `[a, b].as_slice()` of an array | the array |

- **A byte string is its bytes**, written out, as an array of `u8`s is.
- **`as_bytes()` is a copy**, the string's UTF-8 bytes: nothing can write
  through the `&[u8]` it gives, so a copy is what a view of it would be.
  A multi-byte character is more than one byte, as in Rust: `"é"` is `[195,
  169]`.
- **An array's `as_slice()` is the array**, as a `Vec`'s is (ADR 0123).

## Why

- **It's the JS a person writes**: an array of numbers, and `TextEncoder`,
  which is how JS has a string's UTF-8 bytes.
- **It's exact**: `TextEncoder` encodes as Rust's strings are encoded.

## Consequences

- Byte strings and `as_bytes()` compile, compared with native Rust by the
  `byte_strings` corpus case, and `bytes()`, by `str_bytes`. (Amended:
  `bytes()` was an error, which the pilot worked around.)
- A C string, `c"..."`, and `str::from_utf8` are still errors.

## Amendment: a text's bytes are lent, and a long text's facts are kept (#1184, #1183)

Case: A, the user's decision (option 1 of three, 2026-11-10); and D for
the address of the bytes.

**Context.** `as_bytes()` encoded the whole text each time it was asked,
and `len()` and `&s[i..]` counted it from its start. Rust's are free, so a
parser asks them each step, `input.as_bytes().get(i)` in a loop, a
cursor's `peek()`, `while i < s.len()`: each step cost the text's length,
and the loop was quadratic. semver's `identifier` of its 5.6 MB test text
ran past the Crater's 20-minute timeout (`test_identifier.rs::test_new`);
a 60,000-character cursor didn't end in 10 s, where native Rust took
milliseconds. The cost is the program's, not the test's: a real app
reading a long text a byte at a time had it.

The options were: keep a long text's facts in the runtime, by its text,
`as_bytes()` lent as one shared array; hoist a loop's `as_bytes()` before
it, which is what a person writes but leaves a method called each step, a
cursor's `peek()`, and `len()` quadratic; or change nothing and give
semver's file a shorter timeout.

**Decision.**

| Rust | JS |
|---|---|
| `s.as_bytes()`, `s.bytes()`, `AsRef<[u8]>` of text | `$utf8Bytes(s)`: lent, frozen |
| `s.as_bytes().to_vec()`, `collect()`, `into()`, `try_into()` | a copy of it, `$utf8Bytes(s).slice()`, as of any slice lent |
| `s.into_bytes()`, `into_boxed_bytes()`, `Vec::from(s)` | `Array.from(new TextEncoder().encode(s))`: owned |
| `ptr::eq(s.as_bytes(), ..)`, `s.as_bytes().as_ptr()` compared or kept | rust-js's error: "a pointer into what JS may hold a copy of" |

- **A long text's facts are kept by its text** (`src/runtime/byte_len.js`):
  its length in bytes, whether it's all ASCII, the byte and unit each step
  of 64 bytes starts at, and its bytes, each made where it's first asked;
  of a part `&s[a..b]` that's long itself, where it is in its text, so a
  part of a part, `rest = &rest[n..]`, costs no count of its own. JS's
  text is a value, never changed, so what's kept of a text is always its
  own, whichever string has it. A text of fewer than 64 UTF-16 units is
  counted each time, in fewer steps than a lookup. **At most 8 texts'
  facts are kept** (`$textKept`), the one asked longest ago forgotten
  first: a loop that reads two texts at once, a comparison's, keeps both.
- **`as_bytes()` is lent, frozen**: nothing writes through a `&[u8]`, and a
  write to the array would be one to every reader of the same text's
  bytes, so it's an error (a `TypeError` in a module), never a change.
  Where the bytes are owned, the copy is the one a slice lent has
  (`iterators::made_array`'s answer: `$utf8Bytes` makes no array of its
  own), `to_vec()`'s `.slice()`, a collect's own array. A `&mut` of them,
  `as_bytes_mut()`, `as_mut_vec()`, is refused as before.
- **The bytes have no address of Rust's.** Two `String`s of the same text
  are two places in Rust and one value in JS; with the bytes kept by their
  text, `ptr::eq(s.as_bytes(), t.as_bytes())` of two equal texts would be
  `true` where Rust's is `false`, and without, `ptr::eq(s.as_bytes(),
  s.as_bytes())` was `false` where Rust's is `true` (#1183, a wrong answer
  before this). A record of where the bytes are, as a copied subslice is
  placed within its array (#1185), needs a place to record them in, which
  a JS string, a value, hasn't: case D. A reference `as_bytes()` lends is
  among `Pointed::copies`, so `&raw const *b`, `ptr::eq`'s, is refused; and
  a slice's `as_ptr()` of one, and text's own `s.as_ptr()`, is refused
  unless it's kept within its bytes (`pointers/into_copies.rs`): it and each
  pointer it's stepped to, `add`, `sub`, `cast`, read through, `*p`,
  `p.read()`, what `ptr::copy` copies from, and compared only with another
  of them, `p < end`, `p.offset_from(start)`, as a byte-at-a-time search
  does, which reads and compares as Rust's do. Written through, kept in a
  place, returned or compared with any other pointer, it's refused. Text's
  `s.as_ptr()` was refused where it was stepped at all; it's this rule's
  now (corpus case `text_bytes_pointer_within.rs`).

**Consequences.**

- semver's `test_identifier.rs` as JS: `test_new` in 0.9 s, past 20
  minutes before; the 60,000-character cursor in 52 ms, past 10 s before.
  The corpus case `long_text_read_byte_by_byte.rs` reads texts of 300,000
  bytes each way a parser does, semver's `identifier`, a cursor, `while i <
  s.len()` and a UTF-8 decoder's `&s[i..i + w]`, in both builds against
  native Rust: a return to a cost of the text's length each step would take
  minutes, past the corpus's 10 s for a case.
- `test/runtime-package.test.ts` holds the facts to counting from the
  start at every byte of texts of each width and of their parts, and the
  cap of 8; `test/diagnostics.test.ts` the refusals.
- Still of the text's length each step: `s.chars().next()`, which makes
  every `char` of the text (`Array.from(s)[0]`).
