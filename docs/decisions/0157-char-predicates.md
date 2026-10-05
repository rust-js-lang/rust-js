# 0157. A closure, a function or a set of `char`s as a pattern, and a string's pieces by bytes

Status: Accepted. Extends [0063](0063-text.md), [0138](0138-string-byte-counts.md)
and [0150](0150-string-patterns.md).

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
