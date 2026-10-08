# 0183. Strings and `char`s order by code point, and `trim` removes Unicode's White_Space

Status: Accepted. Amends [0034](0034-strings-and-chars.md),
[0036](0036-iterators-and-sorting.md) and
[0182](0182-boundary-matrices.md).

Case: C, A ([0262](0262-when-rust-and-js-disagree.md)).

## Context

Two of the semantics page's differences were JS's, kept for its look:

- **JS's `<` compares UTF-16 units,** Rust's code points. They agree but
  where the first difference is a surrogate, half of a character past
  U+FFFF, against a unit from U+E000: by code point the character past
  U+FFFF is above, by unit below. `"🦀" < "\u{fffd}"` is false in Rust and
  true in JS; a `sort()`, a `BTreeMap`'s keys, `'a'..='\u{ffff}'` holding
  `'🦀'`, differed with it.
- **JS's `trim()` removes its own whitespace,** U+FEFF but not U+0085.
  Rust's removes `char::is_whitespace`, Unicode's White_Space: U+0085 but
  not U+FEFF.

topcoat, compared in ADR 0182, has both as Rust does. A program that sorts
names, or trims a line read from a file, gave another answer than native
Rust's, for text no one would think to test.

## Decision

**Strings and `char`s order by code point, and `trim`, `trim_start` and
`trim_end` remove Unicode's White_Space, as Rust's do:**

| Rust | JS |
|---|---|
| `a.cmp(&b)`, `a.max(b)`, `v.sort()`, `BTreeMap<String, _>` | `$cmp(a, b)`, `v.sort($cmp)` |
| `a < b`, `c >= d`, `'a'..='z'` in a pattern or `contains` | `$cmp(a, b) < 0` |
| `a < "m"`, `c >= 'a'` | `a < "m"`, `c >= "a"`: a literal below U+D800 |
| `v.binary_search(&s)` | `$binarySearchBy(v, (each) => $cmp(each, s))` |
| `s.trim()`, `s.trim_start()`, `s.trim_end()` | `$trim(s)`, `$trimStart(s)`, `$trimEnd(s)` |

- **`$cmp` finds the first unit that differs,** and swaps the order only
  where one is a surrogate and the other from U+E000; elsewhere it's `<`.
- **A literal below U+D800 keeps JS's operator,** as its units are code
  points and no unit below U+D800 orders differently: `name < "m"` reads as
  written.
- **`$trim` is a regular expression of `\p{White_Space}`,** the property
  `char::is_whitespace` is.

## Why

- **The answer is Rust's, for every string:** the matrices of ADR 0182 now
  print `cmp`, `<` and `max` of each pair, and `trim` of each value, where
  they printed `listed`.
- **It's two fewer differences to learn,** on the page a reader checks
  before trusting the output.

## Alternatives

- **Keep them listed:** `a < b` and `s.trim()` read as written, but a sort
  of user text is wrong where an emoji meets a private-use character, and a
  line's U+0085 stays.
- **Compare arrays of code points,** `[...a]`: exact, but it copies both
  strings for each comparison.
- **`localeCompare` or `Intl.Collator`:** a language's order, not Rust's.

## Costs

- **A comparison of two strings is a call,** `$cmp(a, b) < 0`, a loop where
  `<` was the engine's; a sort of strings passes `$cmp`.
- **`trim` is a helper,** `$trim(s)` where it was `s.trim()`, and a regular
  expression where it was the engine's method.
