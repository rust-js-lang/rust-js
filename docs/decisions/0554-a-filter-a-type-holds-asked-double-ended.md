# 0554. A filter a type holds, asked to be double-ended, is made knowing its back

Status: Accepted. Extends [0545](0545-a-map-stepped-from-its-back-is-made-knowing-it.md).

Case: C ([0262](0262-when-rust-and-js-disagree.md)): what a holder's
`DoubleEndedIterator` steps from its back is Rust's.

## Context

`either`'s `Either<L, R>` is double-ended where `L` and `R` are, and steps
each from its back by its impl. `Either<impl DoubleEndedIterator, ..>`
of `(0..10).filter(p)`, `rev()`d, was refused: "the `DoubleEndedIterator`
of a `Filter<Range<u32>, _>`". The filter was made where nothing stepped
it from its back, so a JS helper that doesn't know its back, and ADR
0545's analysis, which makes a map or a filter knowing its back where the
crate steps it so, saw only `next_back()` and the like called on the
filter itself.

## Decision

**A call that asks a type to be double-ended, a `where X:
DoubleEndedIterator` of its callee's, `rev()`'s among them, steps from
its back each map or filter `X` holds, seen through `impl Trait`, which
is made knowing its back where it's made, `$filterEnds`, `$mapEnds`.**

- Not through a std adapter, whose own call steps what it holds:
  `filter(p).map(f).rev()` is `rev()`'s, as it was.
- `back_stepped_adapter` is the one answer to which adapter types the
  analysis lists.

## Why

`filters_held_double_ended` steps a filter and a map an enum holds from
their back by its impl, `rev()`'d and by `next_back()`, against native
Rust; `either`'s probe runs as native.

## Consequences

- either runs whole.
- Mutations leave what's held unlisted, the `impl Trait` unseen, walk
  through a std adapter, and list the type asked itself.
