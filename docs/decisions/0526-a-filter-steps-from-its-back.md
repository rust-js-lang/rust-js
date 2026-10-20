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
