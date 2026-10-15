# 0416. `RangeBounds` in generic code: a dictionary of the range's bounds

Status: Accepted. Extends [0049](0049-traits-and-generics.md), as
[0162](0162-generic-as-ref.md) and [0167](0167-borrow.md) give `AsRef` and `Borrow`
theirs.

Case: N ([0262](0262-when-rust-and-js-disagree.md)): a range's bounds are
Rust's, what it holds.

## Context

Generic code that takes any range, `R: RangeBounds<usize>`, asks its
bounds, `range.start_bound()` and `range.end_bound()`, each a `Bound<&T>`:
arrayvec's `drain` and `ArrayVec::drain(..)`, and std's own `Vec::drain`
written in a crate. A call of a std trait's method in generic code is its
dictionary's, and `RangeBounds` had none: the call was refused.

## Decision

**A range's `RangeBounds` is a dictionary of its two bounds, each a
`Bound` of what its field holds, made where the range's type is known.**

```js
function start(r, RRangeBoundsUsize) {
  const value = RRangeBoundsUsize.start_bound(r);
  ..
}
start({ start: 2, end: 5 }, {
  start_bound: (range) => ({ TAG: "Included", _0: range.start }),
  end_bound: (range) => ({ TAG: "Excluded", _0: range.end }),
});
```

| Range | `start_bound` | `end_bound` |
|---|---|---|
| `a..b` | `Included(a)` | `Excluded(b)` |
| `a..` | `Included(a)` | `Unbounded` |
| `..b` | `Unbounded` | `Excluded(b)` |
| `a..=b` | `Included(a)` | `Included(b)` |
| `..=b` | `Unbounded` | `Included(b)` |
| `..` | `Unbounded` | `Unbounded` |
| `(Bound<T>, Bound<T>)` | its first | its second |

- **A `Bound` is made as any of its values is**, `adt_value`'s, the
  dictionary spelling no representation of its own; its type is the
  trait's method's, `Bound<&T>`, whose reference is what it refers to.
- **Which bounds a range has is `recognition::range_bounds`'s**, by its
  lang item, and `RangeBounds` is a trait generic code calls through a
  dictionary, `operational`'s.

## Why

- **Rust's bounds, each range's**: `test/corpus/range_bounds_generic.rs`
  asks every range of std's and a pair of `Bound`s, natively and in JS.

## Consequences

- **A `RangeInclusive` iterated to its end** has an `end_bound` of
  `Excluded` in Rust, which rust-js's `{ start, end }` doesn't keep: its
  `Included` is said; generic code that asks the bounds of a spent
  inclusive range is rare.
