# 1002. A range stepped already is made knowing its length as its range

Status: Accepted. Fixes a wrong answer my #383 change (593155fa5)
introduced, and closes two of #385's gaps.

Case ([0262](0262-when-rust-and-js-disagree.md)): results change.

## Context

593155fa5 made an `enumerate()` made in place of what isn't an array's
items `$enumerate` of its source made knowing its back and length
(`as_exact_size`). Of a range, `(1..5).enumerate()`, that source is the
range stepped from its start already, `$iterator({ start: 1, end: 5 },
$rangeNext)`. `as_exact_size`'s range rule took that for the range and
stepped it as one again:

```rust
fn counted<I: DoubleEndedIterator>(mut it: I) -> (bool, usize) { (it.next_back().is_some(), it.count()) }
counted((1..5u32).enumerate())
```

Rust gives `(true, 3)`; the JS gave `(false, 0)`. It was refused before
593155fa5.

Two gaps of #385:
- A `rev()` of a `skip()` given as an `ExactSizeIterator`,
  `first((0..10).skip(6).rev())`. Its source was `$skipping(source, n)`,
  made by `as_exact_size` already, which it didn't read.
- `size_hint()` of a map of a type parameter given as it is, `I: Clone`'s.

## Decision

- `range_object`: a range's rules in `as_exact_size` and
  `as_double_ended` take one stepped from its start by its own step,
  `$iterator(range, $rangeNext)`, as the range.
- `as_exact_size` takes `$skipping(source, n)` or `$taking(source, n)` as
  it is: only it makes those two-argument forms, knowing their back and
  length. `closing.rs`'s, given a drop, is not one of them.
- A map's `size_hint()` of a type parameter's source is `$sizeHint` of
  what `$given` made of it, which has its dictionary's.

## Consequences

`test/corpus/lengths_and_backs_given_on.rs` checks each against native
Rust. The mutations are `stepped-range-restepped`, `made-skipping-reread`
and `generic-map-hint-refused`.

Still refused, listed on #385:
- `next_back()` of a generic `Copied<I>`;
- a map whose closure has a destructor, given as an `ExactSizeIterator`,
  which a MIR temporary holds.
