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


## Amendment (2026-11-12, #1501): a closure is asked as std's searcher asks it

Case ([0262](0262-when-rust-and-js-disagree.md)): results change; Rust's
behavior preserved. A closure pattern may print, panic or count its calls,
so which `char`s it's asked of, in what order, is the program's to see.
`split_once` asked it of every `char`, and `rsplit_once` searched from the
front, where std's `find` and `rfind` ask each `char` from the front, or
from the back, and stop at the first that holds. `$splitOnceBy` now
searches so. The iterators of a closure pattern, `split` and its kin,
`matches`, `match_indices`, follow in their own commit under #1501. Corpus
case `str_split_once_by_a_closure_searches_as_std`; mutation
`split-once-asks-every-char`.

Then (2026-11-12, #1501 part 2): **a `str`'s searches by a pattern that runs
code are std's iterators, asked as they're stepped.** `split`, `splitn`,
`rsplit`, `rsplitn`, `split_terminator`, `rsplit_terminator`,
`split_inclusive`, `matches`, `rmatches`, `match_indices` and
`rmatch_indices` of a closure, a function of the crate's or a function
pointer (`recognition::pattern_runs_code`, keyed on the pattern's type, the
coordinator's call) are `$lazySplitBy` and the like: JS iterators that ask
the pattern of each `char` as a piece is stepped, from the front or from
the back, the two ends meeting, as std's `CharPredicateSearcher` and
`SplitInternal` do (`allow_trailing_empty`, `finished`, the inclusive steps,
`SplitN`'s count), its hint std's default, `(0, None)`. Its form has a back
wherever std's is double-ended, so its own `next_back()` steps it
(`has_a_back`); what's made of one, a `map`, is made knowing its back only
where the crate asks a search whole, its back, its `Debug`, a `dyn` of one
(`searches_whole`, the rule `chars_whole` is for a `Chars`, one walk of
`analysis/chars_asked.rs` asked of each kind, `TextIter`), else JS's own,
`.map(f)`, as of a string's `chars()`. They were the array of what the search
found, the pattern asked of every `char` before the first piece: a printing
closure printed out of order, a `take(1)` asked it of the whole string, and
a panic came before any piece. A `char`, a `&str`, a set of `char`s and
std's own predicates run no code: their searches stay the array, now
`Array.from` of the same iterator, so one implementation answers both, and
their output is unchanged. Five corpus snapshots and two examples' change: `Iterator.from(..)` and
`.values()` of a closure's search are the iterator itself, a `for` steps it
as it did, and a `collect` of one, `splitn_by`'s, `calc`'s and `text`'s, is
`Array.from($lazySplitBy(..))`. Corpus
case `str_searches_ask_a_closure_as_they_step` against native Rust in both
builds; ten mutations, caught, and six of the eager helpers' re-anchored to
the one implementation, caught. A slice's `split` family and `chunk_by`,
whose predicate is always the program's, follow under #1501.
