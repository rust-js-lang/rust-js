# 0526. A `filter` steps from its back

Status: Accepted. Extends [0455](0455-stepping-from-the-back.md) and
[0429](0429-a-double-ended-iterator-knows-its-back.md); accepts part of the
[iterators draft](draft/cloned-and-buffering-iterators.md).

Case: C ([0262](0262-when-rust-and-js-disagree.md)): the items kept, and
the order the test is asked of them, are Rust's `Filter::next_back`'s.

## Context

A `map` of an iterator that knows its back knows its own, `$mapEnds` (ADR
0455). A `filter` didn't: generic code's `it.filter(p).next_back()`, its
`rev()` and its `rfold`, were refused ("a `Filter<I, ..>` as a
`DoubleEndedIterator`, which rust-js can't step from its back"), as was a
`filter_map`'s. itertools' `FilterOk::rfold` and `FilterMapOk`'s were the
crate's last refusals once `tee` compiled
([0523](0523-libraries-share-one-counted-rc.md)).

## Decision

**A `filter` of an iterator that knows its back is `$filterEnds(it, p)`,
which knows its own: from either end, each item `p` keeps, `p` asked of
each as it's reached.** A `filter_map` is the same, a map filtered:
`$filterEnds($mapEnds(it, f), present)`, each `Some` that looks like
`None` taken out of its box by another `$mapEnds`.

- **One answer to which adapters step from their back as their source
  does**: `back_stepped_source`, of a `Map`, a `Filter`, a `FilterMap` or a
  `Copied`, which `knows_its_back` asks.
- **Made so wherever its type knows its back**, generic code's, and made so
  where it's stepped from its back in the expression that makes it,
  `xs.iter().filter(p).rfold(..)`, as a `map` is.
- **Of items with a destructor**, which a `filter` drops as it passes them,
  not yet: the JS helper, as before.
- `$mapEnds` and `$filterEnds` are JS iterators as they're made, in
  `ITERATOR_HELPERS`: nothing wraps them in `Iterator.from`.

A `filter` or a `map` a variable holds before it's stepped from its back,
of a source that's not generic code's, is still refused, as before; the
draft keeps it.

## Why

`filter_ends` steps generic code's `filter` and `filter_map` from the back
by `next_back`, `rev` and `rfold`, from both ends in turn, a test that
prints as it's asked, a `Some` of an `Option`, and one made where it's
folded from its back, against native Rust.

## Consequences

- itertools' next refusal is `{:?}` of an `Enumerate`.
- Mutations keep every item, drop the back, stop seeing a `filter` or a
  `filter_map` as stepped from its back, make either a JS helper, and
  leave a `Some` boxed.

## Amendment: a `filter_map` known to step from its back is made so in one place (2026-11-03, #491)

`combinators/adapters.rs` made a `filter_map` of a source that knows its
back as `$filterEnds($mapEnds(..))` where it was written. Since 9bea53c20
(ADR 0545, amended), `as_double_ended` makes a `filter_map` written in
place, `src.map(f).filter(present)`, knowing its back where it's stepped
from there. So the earlier arm was a second way to the same JS. The
nightly's mutation `filter-map-ends-native`, which turns it off, survived
because the corpus's JS and output were the same without it. The arm is
gone, and with it that mutation and `filter-map-ends-boxed`.
`filter_ends.rs`, `held_filter_map_from_its_back.rs` and the rest of the
corpus give the same JS and agree with native Rust.

## Amendment: its front steps by its source's `find` (#1226)

A filter made knowing its back, `$filterEnds`, stepped its source's `next`
for each of its own steps from the front. Std's `Filter::next` asks its
source's `find`, a `try_fold`, and for most sources the two are the same.
They differ where the source's `try_fold` is its own: `Rev::find` is its
source's `rfind`, and a `take(n)`'s `try_rfold` trims its back by
`nth_back(len - n - 1)`. Its `next_back` trims by `nth_back(len - n)`, one
slice dropped front to back. So a filter of `v.into_iter().take(3)`
reversed, made knowing its back as generic code that steps it from either
end makes it, dropped what the take trims as 4 then 5, where Rust drops 5
then 4. Results changed, so Rust's behavior is kept (ADR
[0262](0262-when-rust-and-js-disagree.md)'s third case).

The front step is now `$Steps`' own `filter`'s (`$filterDropping`), the
one owner of `Filter::next`, which already asked its source's `tryFold`.
`$filterEnds` adds its back to it and no longer restates the front.
`test/corpus/filter_of_a_take_reversed_trims_by_its_try_rfold.rs` holds it
against native Rust in both builds, and the mutation
`filter-ends-steps-by-next` holds the line.
