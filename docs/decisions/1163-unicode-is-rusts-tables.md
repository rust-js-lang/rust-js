# 1163. A `char`'s Unicode answers are Rust's own tables

Status: Accepted (2026-11-15, #1888, the lead's decision). Amends
[0063](0063-text.md), [0183](0183-code-point-order-and-trim.md) and
[0327](0327-cell-deque-char-methods.md).

Case ([0262](0262-when-rust-and-js-disagree.md)): results change, so
Rust's behavior is kept. `\p{Cc}` stays JS's: no case, as Unicode's
stability policy never changes which `char`s are `Cc`.

## Context

`char`'s questions were JS's regular expressions of the property Rust
asks, `/^\p{Alphabetic}$/u.test(c)` (ADR 0063), `{:?}`'s escapes were
`\p{Cn}`, `\p{Grapheme_Extend}` and the rest, and `to_uppercase()` was
`toUpperCase()`. Their tables are the JS engine's, of its Unicode
version: node 24.22's is 17.0, each browser has its own, and Rust
1.100's is 18.0. The guide said so, but ADR 0262 lists no such
exception, so the answers were wrong where the versions differ:
Rustlantis seed 12462637's `{:?}` of `'\u{3f5cd}'`, which Unicode 18
assigns, was `'\u{3f5cd}'` where native Rust prints `'𿗍'`, and its
`is_alphabetic()` was `false`. Between 17 and 18 alone, 12,833 `char`s
answer `is_alphabetic` otherwise, 323 `is_numeric`, 145 `is_lowercase`,
and 21 `to_uppercase` mappings differ, U+0277's among them.

## Decision

**The runtime asks Rust's tables,** `crates/rust-js-program/src/runtime/unicode_tables.js`,
which `bun scripts/unicode.ts` makes of the pinned toolchain's std:
scripts/unicode/main.rs, built by the checkout's toolchain, asks std's
own methods of every `char`, `is_alphabetic()`, `escape_debug()`,
`to_uppercase()`, and the unstable `is_cased()` and
`is_case_ignorable()` that `str::to_lowercase`'s Final_Sigma asks. So
each table is what std answers, not a copy of how core stores it.

| Rust | JS |
|---|---|
| `c.is_alphabetic()`, `is_numeric()`, `is_alphanumeric()` | `$isAlphabetic(c)`, `$isNumeric(c)`, `$isAlphanumeric(c)` |
| `c.is_uppercase()`, `is_lowercase()`, `is_whitespace()` | `$isUppercase(c)`, `$isLowercase(c)`, `$isWhitespace(c)` |
| `c.is_control()` | `/^\p{Cc}$/u.test(c)`, as it was |
| `s.to_uppercase()`, `s.to_lowercase()` | `$toUppercase(s)`, `$toLowercase(s)` |
| `c.to_uppercase()` | `$toUppercase(c)`'s `char`s |
| `s.split_whitespace()` | `$splitWhitespace(s)`, an array |
| `s.trim()`, `{:?}`, `escape_debug()` | `$trim(s)`, `$debugStr(s)`, as they were, of the tables |

- **Each table is a string of steps**, the boundaries of its ranges each
  a base-36 step from the last, a case map's runs of `char`s the same
  offset and stride apart; a table is made into a regular expression's
  class, or a map, the first time it's asked, and only then. Each is a
  constant of its own, so a bundle keeps only those its program asks.
  They're 21 KB of `@rust-js/std`'s `runtime.js`.
- **ASCII text is cased by JS's own `toUpperCase()`,** as Rust's own
  first converts its ASCII prefix: no Unicode version maps ASCII
  otherwise.
- **`to_lowercase`'s `Σ` is Rust's Final_Sigma**: `ς` where a cased
  `char` comes before it and none after, past those case-ignorable.
- **A Grapheme_Extend `char` is escaped wherever it is,** as 1.100's
  `{:?}` and `escape_debug()` are, so `$debugChar` has no `extend`
  parameter left; the generator refuses a std that escapes one first
  alone.
- **The tables follow the toolchain:** `test/unicode.test.ts` fails where
  they aren't what scripts/unicode.ts makes of its std, so a bump that
  moves Unicode makes them again.

## Alternatives

- **An ADR 0262 exception,** the engine's tables: results change across
  engines and versions, on every page.
- **core's own tables, `unicode_data.rs`, copied with its search
  functions:** the same answers, but tied to how core stores them, which
  changes between releases; std's methods are the answers themselves.
- **One range list searched in JS:** what the regular expressions'
  classes are, made by the engine's own matcher, which is faster.

## Consequences

- Output: `s.toUpperCase()` is `$toUppercase(s)`, a helper upstream JS
  doesn't call, so ports differ there; `shared?.toUpperCase()` of an
  `Option` is `shared != null ? $toUppercase(shared) : undefined`.
- test/unicode.test.ts compiles a program that hashes, a plane a line,
  every `char`'s answers, each question, `{:?}` and both case maps, and
  compares them with native Rust's; it isn't a corpus case, whose nets
  compile and run each case many times, as a million `char`s a run is
  seconds of JS. Corpus case unicode_answers_are_rusts (U+3F5CD,
  Final_Sigma's contexts, mappings Unicode 18 added, `split_whitespace`
  and `trim`) against native Rust.
- Mutations of `unicode.js`: alphabetic-by-the-engine,
  unicode-set-boundaries-shifted, unicode-map-run-unstrided,
  uppercase-by-the-engine, final-sigma-skips-nothing; and
  debug-escapes-by-the-engine, str-case-js-own, split-whitespace-js-s
  and trim-by-js-whitespace, retargeted. debug-escapes-halfwidth-voiced-marks,
  debug-shows-default-ignorable, escape-debug-str-marks and
  str-escape-debug-extenders-first-only are gone with the code they broke:
  what they kept is the tables', which the every-`char` case checks.
- Guide: semantics.md's text.
