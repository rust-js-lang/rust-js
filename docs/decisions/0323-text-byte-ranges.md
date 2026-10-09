# 0323. A `String`'s byte-range edits, and `str`'s searches from the end

Status: Accepted. Extends [0138](0138-string-byte-counts.md),
[0149](0149-string-editing.md) and [0150](0150-string-patterns.md); counted by
[0314](0314-std-data-structures.md).

Case: C ([0262](0262-when-rust-and-js-disagree.md)).

## Context

A `String`'s `split_off`, `drain`, `replace_range` and
`extend_from_within`, and `str`'s `split_inclusive`, `rsplit_terminator`,
`rmatches`, `rmatch_indices`, `split_at_checked`, `encode_utf16`,
`floor_char_boundary`, `ceil_char_boundary`, `trim_ascii` and `is_ascii`,
with `String::from_utf16`, were refused. Each counts and checks UTF-8
bytes, and a search from the end finds other matches than one from the
start where they overlap: `"aaa".rmatch_indices("aa")` is `[(1, "aa")]`.

## Decision

**Each is std's, by its UTF-8 bytes and its checks, and an edit gives its
string's place the new string, as ADR 0149's do.**

| Rust | JS |
|---|---|
| `s.split_off(at)`, `s.drain(range)` | `const split = $strSplitOff(s, at); s = split[0];`, its `split[1]`; `$strDrain`'s `char`s |
| `s.replace_range(range, t)`, `s.extend_from_within(range)` | `s = $replaceRange(s, a, b, t)`, `s = $strExtendWithin(s, a, b)` |
| `split_inclusive`, `rsplit_terminator`, `rmatches`, `rmatch_indices` | `$splitInclusive`, `$rsplit`'s without the empty end, `lastIndexOf` from the end |
| `split_at_checked(at)`, `floor_char_boundary(at)`, `ceil_char_boundary(at)` | `$splitAtChecked`, `$floorCharBoundary`, `$ceilCharBoundary` |
| `trim_ascii`, `is_ascii`, `encode_utf16` | `/^[\t\n\f\r ]+|…/`, `/^[\0-\x7f]*$/`, each UTF-16 unit |
| `String::from_utf16(u)`, `from_utf16_lossy(u)` | `Ok` where `isWellFormed()`, `toWellFormed()` |

- **A range's bounds are read once**, by `range_bounds`, as a slice's are:
  `a..b`, `a..=b`, `..`, or one kept as a value.
- **Each panics with std's message**: `slice::range`'s, then
  `replace_range`'s `start of range should be a character boundary`, or
  the other edits' assertions.
- **ASCII whitespace is `u8::is_ascii_whitespace`'s**, without the vertical
  tab JS's `\s` has.
- **A `FromUtf16Error` shows std's**: `FromUtf16Error { kind:
  LoneSurrogate }` and `invalid utf-16: lone surrogate found`, the one kind
  `from_utf16` makes.

## Why

- **It's the same program**: each counts UTF-8 bytes and panics where
  std's does.
- **It's tested**: a corpus case runs each against native Rust, of text
  with two- and four-byte `char`s, an overlapping search from the end, an
  empty pattern, a vertical tab and lone surrogates; another the panic of
  a range inside a `char`. Mutations skip the check, swap `split_off`'s
  halves, keep what `drain` takes, extend by the whole string, drop the
  match `split_inclusive` keeps, keep the empty end, search forward,
  accept a lone surrogate, round a boundary up, and trim JS's whitespace.
- `docs/std-coverage.txt`: `String` 31 of 43, `str` 64 of 83.
