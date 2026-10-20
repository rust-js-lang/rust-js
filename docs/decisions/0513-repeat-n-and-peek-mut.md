# 0513. `iter::repeat_n` and `Peekable::peek_mut`

Status: Accepted. Extends [0422](0422-an-iterator-holding-a-value-is-closed-to-drop-it.md) and
[0071](0071-stepping-iterators.md).

Case: C ([0262](0262-when-rust-and-js-disagree.md)): how many clones are
made, and when a value drops, would change, so Rust's is kept.

## Context

`iter::repeat_n(x, n)` and a `Peekable`'s `peek_mut()` were refused: rust-js
had no form for either. And `$repeating`, `repeat`'s, was written inside
`Iterator.from(..)`, though it makes a JS iterator already.

## Decision

**`repeat_n(x, n)` is `$repeatingN(x, n, clone, drop)`**: `n - 1` clones,
then `x` itself, as std's moves it last; of a count of none, `x` dropped
where it's made, as std's drops it then; and dropped when it's closed
before its last.

**`peek_mut()` is a `&mut` to the item a `Peekable` gives next**: of an
object, the item itself, `$peek(it)`; of a number or text, a handle on it,
`$peekCell(it)`, on its array's place, `$mutAt(items, at)`, or on the step
a lazy one keeps for its next `next`.

`$repeating` and `$repeatingN` are in `ITERATOR_HELPERS`, which the
runtime's test holds, so neither is written inside `Iterator.from`.

## Why

- **It's the same program**: `repeat_n_and_peek_mut` repeats numbers,
  strings and a value whose `Clone` prints, none, all and some of them,
  and writes through `peek_mut` of an array's iterator, a lazy one and
  strings, against native Rust.

## Consequences

- `iterator_sources` and `repeat_drops` lose their `Iterator.from`.
- Mutations drop no value of a count of none, clone the last, drop none
  when closed, write a lazy step's copy, give a number for a `&mut`, and
  take `repeat_n` for `repeat`.
