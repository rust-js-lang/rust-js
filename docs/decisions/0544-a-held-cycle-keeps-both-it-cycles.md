# 0544. A `Cycle` a struct holds keeps both it cycles

Status: Accepted. Extends [0491](0491-cycle-clones-its-iterator.md) and
[0542](0542-a-held-adapter-keeps-its-source-and-state.md).

Case: C ([0262](0262-when-rust-and-js-disagree.md)): a held `Cycle`'s
`Debug`, its clone and an empty one's size hint are Rust's.

## Context

itertools' `CircularTupleWindows` holds a `Cycle<I>`, whose derived
`Debug` shows what it started with and what it steps, `Cycle { orig:
IntoIter([1, 2]), iter: IntoIter([]) }`. `$cycleOf` kept both in its
closure, and a `cycle()` of an array's iterator was a generator, which
keeps neither: its `Debug` and its clone were refused.

## Decision

**`Cycle` is the third held kind (ADR 0542): a held one is `$cycleOf`,
an array's iterator's too, stepped by `$next`, which keeps what it
started with, `orig`, and what it steps, `iter`, on itself.**

- Its `Debug` is std's, `Cycle { orig: .., iter: .. }`, of each as its
  source's type shows it, plain and pretty.
- It carries its clone, `$cloneKey`'s, as `$memchrIter` does (ADR 0525):
  of clones of both, where it is.
- Its size hint is Rust's: `(0, Some(0))` of an empty one, `(0, None)` of
  one whose source's hint starts at none, else `usize::MAX` and no end.
- `iter::Empty`'s `Debug` is `Empty`.

## Why

`held_cycle` is a struct holding a generic `Cycle`, shown before and after
it starts again, cloned, and of an empty iterator with its size hint; and
one holding a `Cycle` of an array's iterator, against native Rust.

## Consequences

- itertools' `CircularTupleWindows` compiles; two refusals remain:
  `PutBack`'s `Debug` and the map folded from its back in a library.
- Mutations make a held array's cycle a generator, refuse its `next`, its
  `Debug`, its clone and `Empty`'s, restart a clone, and give an empty one
  no end.
