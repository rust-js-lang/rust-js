# 0493. A zip of arrays' iterators steps from its back

Status: Accepted. Extends [0164](0164-double-ended-iterators.md) and
[0429](0429-a-double-ended-iterator-knows-its-back.md).

Case: C ([0262](0262-when-rust-and-js-disagree.md)): the pairs, in the
order Rust's `Zip::next_back` gives them.

## Context

`rev()` of a lazy iterator is refused (ADR 0164): its closures would run
in another order. A zip of two arrays' iterators runs none, but was
refused with them: itertools' `MultiProduct` walks
`iters.iter_mut().zip(values.iter_mut()).rev()`.

## Decision

**`rev()` of a zip of two arrays' iterators, made here, is `$zipBack(a,
b)`: the longer trimmed from its back to the shorter's length, then each's
back paired, as Rust's `Zip::next_back` does where both sides are
double-ended and exact-sized.** Of `iter()`'s and `iter_mut()`'s alike,
`$iter`s, which know their backs and lengths. `reversed`, the one place
`rev()` steps from a back, makes it.

- **Not of items with a destructor**, which trimming would drop: refused,
  as before.

```js
for (const [x, y] of $zipBack($iter($mutItems(a)), $iter($mutItems(b)))) {
```

## Why

- **It's std's**: `zip_from_back` reverses zips of a longer first side, a
  longer second, an empty one, and changes items through `iter_mut()`,
  breaking early, with native Rust.

## Consequences

- **What compiles grows**: itertools' `multi_cartesian_product`.
