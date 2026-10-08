# 0150. Splitting and searching by a pattern, as Rust searches

Status: Accepted. Extends [0063](0063-text.md) and
[0138](0138-string-byte-counts.md).

Case: C, D ([0262](0262-when-rust-and-js-disagree.md)).

## Context

`split`, `find` and `split_once` of a `&str` or a `char` pattern compile
(ADRs 0063, 0138). `splitn`, `rsplitn`, `rsplit`, `split_terminator`,
`split_at`, `match_indices`, `matches` and the `trim_*_matches` were errors,
though parsing `key=value` lines is what they're for:

```rust
for part in "k=v;x=y".split_terminator(';') {
    let mut kv = part.splitn(2, '=');
}
```

JS's `split` and `indexOf` search from the start. Rust's `rsplit` searches
from the end, which differs where matches overlap: `"aaa".rsplit("aa")` is
`["", "a"]`, and splitting forwards then reversing gives `["a", ""]`.

## Decision

**Each is a runtime helper that searches as Rust's does, by a `&str` or a
`char` pattern, and the `trim_*_matches` by a closure or a function too:**

| Rust | JS |
|---|---|
| `s.splitn(n, p)` | `$splitN(s, n, p)`: `n - 1` pieces, and the rest whole |
| `s.rsplit(p)`, `s.rsplitn(n, p)` | `$rsplit(s, p)`, `$rsplit(s, p, n)`: searched from the end |
| `s.split_terminator(p)` | `$splitTerminator(s, p)`: no empty last piece |
| `s.split_at(at)` | `$splitAt(s, at)`, as `[&s[..at], &s[at..]]` |
| `s.match_indices(p)`, `s.matches(p)` | `$matchIndices(s, p)`, `$matches(s, p)` |
| `s.trim_matches(p)` | `$trimMatches(s, p)` |
| `s.trim_start_matches(p)`, `s.trim_end_matches(p)` | `$trimMatches(s, p, true, false)`, `$trimMatches(s, p, false, true)` |

- **Offsets are UTF-8 bytes,** as ADR 0138 has them: `match_indices`
  gives where each match starts in bytes, and `split_at` panics as
  `&s[..at]` does, inside a `char` or past the end.
- **An empty pattern matches at each `char`'s ends,** as Rust's does:
  `"ab".rsplit("")` is `["", "b", "a", ""]`.
- **Each gives an array,** an iterator as any array is (ADR 0036), so
  `splitn(2, '=').next()` steps it.
- **Still errors:** a closure or a `char` slice as the pattern of a
  splitter or a search, and a `&str` one for `trim_matches`, which Rust
  refuses too.

## Why

- **It's exact:** the `string_patterns` corpus case compares each with
  native Rust, overlapping `rsplit` and `rsplitn`, empty patterns, and
  offsets past a multi-byte `char` included.
- **It's the JS a person writes:** a helper's name for each method, where
  the plain JS would answer differently.
