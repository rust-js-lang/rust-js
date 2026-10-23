# 0157. A closure, a function or a set of `char`s as a pattern, and a string's pieces by bytes

Status: Accepted. Extends [0063](0063-text.md), [0138](0138-string-byte-counts.md)
and [0150](0150-string-patterns.md).

Case: C, D ([0262](0262-when-rust-and-js-disagree.md)).

## Context

A pattern can be what a `char` is, not only what text is:

```rust
line.find(|c: char| c.is_ascii_digit());
line.split([';', ',']);
name.starts_with(char::is_uppercase);
```

`split` and `contains` took a closure (ADR 0063), but not a function or a
set of `char`s, and `find`, `rfind`, `starts_with` and `ends_with` took
neither. `get(range)`, `is_char_boundary`, `split_ascii_whitespace`,
`len_utf8`, `len_utf16`, `char::from(u8)` and `String::from_iter` were
errors too.

## Decision

**A closure, a function or a set of `char`s as a pattern is a predicate of
one `char`,** which each method tries on each `char` in turn:

| Rust | JS |
|---|---|
| `s.find(\|c\| c.is_ascii_digit())` | `$findBy(s, (c) => /^[0-9]$/.test(c), false)`: the byte it's at |
| `s.rfind(char::is_numeric)` | `$findBy(s, (c) => /^\p{N}$/u.test(c), true)` |
| `s.split([';', ','])` | `$splitBy(s, (c) => [";", ","].includes(c))` |
| `s.starts_with(p)`, `ends_with(p)` | `$startsBy(s, p, false)`, `$startsBy(s, p, true)` |
| `s.contains(p)`, `trim_matches(p)` | as of a closure, a set's `(c) => set.includes(c)` |
| `s.get(a..b)` | `$strGet(s, a, b)`: `&s[a..b]`, or `undefined` where that panics |
| `s.is_char_boundary(i)` | `$charBoundary(s, i) !== undefined` |
| `s.split_ascii_whitespace()` | `s.split(/[\t\n\f\r ]+/).filter((word) => word !== "")` |
| `c.len_utf8()`, `len_utf16()` | `$byteLen(c)`, `c.length` |
| `char::from(b)` of a `u8` | `String.fromCharCode(b)` |
| `b.is_ascii_digit()` of a `u8`, and its other ASCII tests | `b >= 48 && b <= 57`, Rust's ranges (Amended: chrono's parser.) |
| `b.to_ascii_uppercase()` of a `u8` | `b >= 97 && b <= 122 ? b - 32 : b` |
| `char::from_u32_unchecked(n)`, of a code point its caller checked | `String.fromCodePoint(n)` (Amended: an error; it stopped utf8_iter.) |
| `String::from_iter(items)` | `items.join("")`, as `collect()` into a `String` is |

- **Offsets are bytes,** as `find`'s are (ADR 0138): the byte where the
  first `char` that holds starts.
- **`ends_with` tries the last `char`,** an emoji's two UTF-16 units whole.
- **`get` is `None` where slicing panics:** past the end, backwards, or
  inside a `char`.
- **ASCII whitespace is Rust's five,** space, tab, line feed, form feed and
  carriage return, not JS's `\s`.
- **Still errors:** `make_ascii_uppercase` and `make_ascii_lowercase` of a
  `String` in place, and a predicate as the pattern of `strip_prefix`,
  `split_once`, `replace` and `splitn`.

## Why

- **It's exact:** the `text_predicates` corpus case compares each with
  native Rust, offsets past a multi-byte `char`, an emoji, empty strings
  and ranges inside a `char` included; `byte_ascii` each of a byte's ASCII
  tests on all 256 bytes.
- **It's the JS a person writes:** `(c) => [";", ","].includes(c)`, the
  predicate a closure would be.

## Amendment: `splitn` of a predicate

`s.splitn(n, ['e', 'E'])`, rust_decimal's split of an exponent, was
refused. It's `$splitBy(s, matches, n)`, `split`'s helper given its
count: at most `n` pieces, the last what's left whole, a set of `char`s
made the predicate of being one, as `split`'s is. The `splitn_by` corpus
case splits by a set and a closure, of none and of more than there are,
against native Rust.

## Amendment: `matches` and `rmatches` of a predicate

`matches(p)` and `rmatches(p)` of a closure, a function or a set of
`char`s were refused (#118). They're `$matchesBy(s, p)`, each `char` the
predicate holds of, `rmatches` asking it from the end as Rust's does; a
set is the predicate of being one of it, as for `split`. Held by
`test/corpus/matches_by_predicate.rs`.

## Amended (#118)

A predicate of a `char` is a pattern of every `str` method that takes
one: `rsplit`, `rsplitn`, `split_terminator`, `rsplit_terminator`,
`split_inclusive`, `match_indices`, `rmatch_indices`, `split_once`,
`rsplit_once`, `strip_prefix`, `strip_suffix`, `replace` and `replacen`
were refused of one. Each is a `…By` op of `TextOp`, as `split`'s and
`find`'s are, its runtime helper `char_pattern.js`'s, of one primitive,
`$charMatches`, each match one `char`, where it starts in UTF-16 units and
in UTF-8 bytes. A set of `char`s is the predicate of being one of them,
as before. Held by `test/corpus/char_patterns.rs` against native Rust.

