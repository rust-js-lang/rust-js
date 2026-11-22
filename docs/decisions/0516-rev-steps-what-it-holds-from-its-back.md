# 0516. `rev()` steps what it holds from its back

Status: Accepted. Extends [0164](0164-double-ended-iterators.md) and
[0429](0429-a-double-ended-iterator-knows-its-back.md).

Case: C ([0262](0262-when-rust-and-js-disagree.md)): the order what's left
drops in would change, so Rust's is kept.

## Context

`v.into_iter().rev()` of items with a destructor was refused. An array's
`rev()` is a reversed copy, `v.toReversed()`, whose rest, dropped, would
drop back to front: Rust's `Rev` holds the `IntoIter` itself, which drops
what it has left front to back.

And `rev()` of a JS iterator that knows its back was `$iterator(it,
$nextBackSome, true)`: a box made and taken off for each item, where
`back()` gives the item as it is.

## Decision

**`rev()` of what knows its back is `$rev(it)`**: `it` stepped from its
back, its back its front, its length its own, and closed as it is. Of an
array's items with a destructor, `$rev` of its `$iter` closed to drop
what's left, `$rev($closing($iter(v), $dropEach(drop)))`, which drops it
front to back. `closed_adapter` makes it, beside `peekable()`'s `$peekable`
(ADR 0514), and a `Rev` held of an array's items drops through
`closed_on_drop`.

## Why

- **It's the same program**: `rev_drops` steps an array's items from the
  back, consumes, takes from and drops a `Rev`, of a fresh `IntoIter` and
  one stepped already, against native Rust.
- **It reads as it does**: `$rev(it)`, not `$iterator(it, $nextBackSome,
  true)`.

## Consequences

- `rev()` of a lazy map of such items, `v.into_iter().map(f).rev()`, stays
  refused; the tests that held `rev()`'s refusal hold `scan`'s and that
  one's.
- Mutations copy the array reversed, drop none of what a `Rev` holds, step
  from the front, and refuse a held `Rev`.

## Amendment: behind a `filter`, and of a `Rev`

`rev()` of an array's items with a destructor was `$rev` of them alone:
behind a `filter` it was `$rev` of JS's own `filter`, which has no back,
and of a `Rev` it took the items out, `$rest(..).toReversed()`, nothing
closing what was left; each threw (#205). One that knows its back, held
or not, is `$rev(it)`, and tidy's `reversed_in_place`, once a
temporary's chain is in place, turns it inward as `rev_items` does one
it sees: `$rev(c.filter(p))` is `$rev(c).filter(p)`, a `filter` called
from the back on the same items, and `$rev($rev(it))` is `it`
(`rev_of_closed_items`, and four mutations). `try_rfold`'s
`$rev($iter(xs))` reads `$iter(xs.toReversed())` so too. The readability
measure's `$iter` share grows by the new case's iterators, each closed to
drop what it doesn't give, as any of items with a destructor is.

## Amendment: one rule of `$rev($iter(v))`

`stepped_in_place` (crates/rust-js-lower/src/iterators/in_place.rs) wrote `$rev($iter(v))`
stepped till it ends as `$iter(v.toReversed())`, which #205's tidy pass
"reversed inward" already does to every `$rev($iter(v))`, so nothing reached
it: the nightly's mutation `rev-of-items-kept-in-place` survived (#228).
`reversed_inward` is the one rule; `rev-of-items-not-reversed-inward` holds it.

## Amendment: a `rev()` stepped from its back isn't reversed inward (2026-11-15, #1855)

Tidy's "reversed inward" turns a `$rev(c)` read in place into a chain over
the reversed items, which a step from the front reads as `$rev(c)` does.
A step from the back can't: JS's own chain has no back. So
`words.into_iter().map(f).rev().rfold(..)`, `$foldBack($rev(..))`, threw
"it[by] is not a function". The same was true of `$nthBack`,
`$advanceBackBy`, `$findBack`, `$tryRfoldOf` and `$nthBackOf`, and of a
`$rev` given a `rev()`, as `try_rfold` is `$tryFoldSome($rev(it))`: that
`$rev` steps the `rev()` it's given from its back. Each of those leaves
the `rev()` it's given as `$rev(..)` now. Backs.rs owns which helpers step
what they're given from its back (`stepped_from_its_back`); the tidy pass
asks it, before it reverses the others inward.

Case ([0262](0262-when-rust-and-js-disagree.md)): results change, as JS
threw. `test/corpus/reversed_map_stepped_from_its_back.rs` holds it
against native Rust: `rfold`, `nth_back`, `rfind` and `try_rfold` of a
`rev()` of a `Vec<String>`'s map, of a slice's and of a `Vec<D>`'s. The
matrix's `map made` adapter (#1826) found it. Mutations
`back-stepped-rev-reversed-inward` (`lower/mir/tidy/stages.ts`) and
`rev-no-back-stepper` (`lower/iterators/mir_steps/backs.ts`).
