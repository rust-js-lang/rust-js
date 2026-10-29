# 0129. A range is a value: `{ start, end }`, iterated as its items

Status: Accepted. Extends [0036](0036-iterators-and-sorting.md) and [0128](0128-iterator-sources.md).
Amended: a range's `size_hint()`, and an inclusive one's `len()` (below).

Case: C, D ([0262](0262-when-rust-and-js-disagree.md)).

## Context

A range was an iterator only where it was written: `for i in 0..n` and
`(0..n).map(f)`. Kept in a variable, passed, or held in a field, `0..n`
was already `{ start: 0, end: n }`, as any struct is, but nothing could
use it: iterating it, `r.contains(&x)`, `r.len()`, `{:?}`, `==` and
`clone()` were errors, and so was `1..=n` as a value. `(1..).take(3)`
compiled and threw, its object taken for items, and `('a'..'e')` was no
items at all.

## Decision

**A range is the object of its bounds, and wherever an iterator is taken
from it, its items:**

| Rust | JS |
|---|---|
| `let r = 2..7;`, `1..=6`, `4..`, `..=4` | `{ start: 2, end: 7 }`, `{ start: 1, end: 6 }`, `{ start: 4 }`, `{ end: 4 }` |
| `r.collect()`, `r.map(f)` | `$range(r.start, r.end)`, `.map(f)` |
| `(1..=n).filter(p)` of a value | `$range(r.start, r.end + 1).filter(p)` |
| `(1..).take(3)` | `$rangeFrom(1).take(3)`, a JS iterator |
| `for i in r` | `for (let i = r.start; i < r.end; i++)` |
| `for n in 1..` | `for (let n = 1; true; n++)` |
| `('a'..='c').collect()` | `$charRange("a", "c", true)` |
| `r.contains(&x)` | `r.start <= x && x < r.end` |
| `r.len()`, `r.is_empty()` | `Math.max(0, r.end - r.start)`, `!(r.start < r.end)` |
| `r.start()`, `r.into_inner()` of `a..=b` | `r.start`, `[r.start, r.end]` |
| `&v[r]`, `&v[..=1]`, `v.drain(r)` | `$slice(v, r.start, r.end)`, `$slice(v, 0, 2)` |
| `{:?}` | `` `${r.start}..${r.end}` `` |
| `r == s`, `r.clone()` | `r.start === s.start && r.end === s.end`, `{ start: r.start, end: r.end }` |
| `r.next()`, `r.next_back()` of a `Range` | `$rangeNext(r)`, `$rangeNextBack(r)`, which move `r.start` or `r.end` |

- **The bounds are read once**: a range read more than once is a `const`
  first, unless it reads the same each time; a `{ start, end }` written in
  place gives its bounds themselves.
- **A `for`'s end is read each time round only if nothing can change it**,
  as with any `for`: else it's a `const` first.
- **A range is changed in place** by assigning to a bound, as any
  struct's field (ADR 0020), and by `next()` and `next_back()`, which take
  `&mut` of it: a range of a type something changes is copied by
  `clone()`, and only then. A `for`'s own stepping isn't a change.
- **A range given where a generic `I: Iterator` goes is its items**; where
  any other `T` goes, it's the range.
- **`char`s step past the surrogates**, as Rust's do: `'\u{d7ff}'` is
  followed by `'\u{e000}'`.
- **`a..` counts on past its type's end**, both iterated and stepped by
  `next()`.
- **Still errors:** `next()` of a `RangeInclusive` kept as a value, which
  Rust marks as having reached its end with a field rust-js's object has no
  place for; iterating through a `&mut` to a range, which steps it; and a
  range of anything but numbers or `char`s as an iterator.

## Why

- **It's the JS a person writes**: an object of two numbers, `start <= x
  && x < end`, and a `for` from `r.start` to `r.end`.
- **It's exact**: the items, the tests, the order the bounds are read in,
  and what `next()` changes are Rust's.

## Consequences

- Ranges as values compile, compared with native Rust by the
  `range_values` corpus case. The literal ranges' JS is unchanged.
- `('a'..'e')` and `('a'..='c')` written in place were no items; they're
  their `char`s now.
- `size_hint()`, `by_ref()` and `RangeInclusive`'s `next()` are still errors.

## Since

- **A range a set or a map is made of, or extended by, is its items**
  (ADR 0344): `new Set($range(1, 7))`, where it was the range's object.

## Amendment: how many are left

A range's `size_hint()` (2026-10-15) is how many numbers it has left,
exactly, as both bounds, `[n, n]`; an inclusive range's `len()` is that
count too, as an exclusive one's was. Both are `range_count`'s, one place:
`Math.max(end - start, 0)`, one more of `a..=b`, none once stepping has
passed its end or where it ends before it starts. Of BigInts, an error.
The corpus's `range_size_hint.rs`, and rustc's `range/range_inclusive.rs`,
pass.

## Amendment: an `a..` steps as release Rust's does (2026-10-29, #414)

An `a..`'s `next()` was `range.start++`: of a `char`, a string, `NaN`; of
an integer, a JS number or BigInt that never wraps, `255u8..` giving
`256`, `i64::MAX..` going past it. Results change, so Rust's behavior is
kept (ADR 0262): **an `a..` of integers steps by its type's bounds,
`$rangeFromNext(bytes, 0, 255)`, past the last to the first, as release
Rust's `Step::forward` wraps (ADR 0011); of a `char`, by its code point,
past the surrogates (`$charStep`), and past `char::MAX` std's panic,
``overflow in `Step::forward` ``, before it gives one.** Its `next` as a
value, an `Iterator`'s, `$iterator({ start: 254 }, $rangeFromStep(0,
255))`, is the same step of those bounds, which `$rangeHintOf` knows as an
`a..`'s, `usize::MAX` and none known (ADR 0918). The bounds are the one
answer of `Num::range`, given by `range_next` and `range_stepper`, which
each `next()` and each `Iterator` of a range asks; `step_by` and `skip`
of one step through it. Held by `range_from_wraps.rs` against native Rust
(`u8`, `i8`, `u16`, `i32`, `i64`, `u128`, `char`, taken, stepped by,
skipped, a `for` over it) and the mutations `range-from-unwrapped` and
`range-from-char-unchecked` (`runtime/range_from_next.ts`) and
`range-from-bounds-dropped` (`lower/std_types/range.ts`). A
`RangeInclusive` stepped past its type's last, `127i8..=127`, whose
start Rust wraps and marks exhausted, is #414's other half.

## Amendment: a range lent to `count()` or `last()` is used up (2026-10-29, #414)

A range's own `count()` and `last()`, of its bounds (#163), left it as it
was: right of a range by value, which nothing reads after, but of one lent,
`r.by_ref().count()`, `(&mut r).last()`, which std's `fold` of a `&mut`
steps to its end, the next `r.next()` gave a number where Rust's gives
`None`. Results change, so Rust's behavior is kept (ADR 0262): **of a
range lent, `range_own_step` asks `$rangeCountOut` and `$rangeLastOut`, and
their inclusive forms, which give what `$rangeLen` and `$rangeLast` give
and move its start to its end, past it of `a..=b`.** Held by
`range_lent_used_up.rs` against native Rust and the mutations
`range-lent-count-kept` (`lower/std_types/range.ts`),
`range-count-out-kept` and `range-inclusive-last-out-kept`
(`runtime/range_len.ts`). An inclusive one's `{:?}` after it, `3..=3
(exhausted)` where JS has `4..=3`, is #414's other half.
