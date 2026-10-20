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
