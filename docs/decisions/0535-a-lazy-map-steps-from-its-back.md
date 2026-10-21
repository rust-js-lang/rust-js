# 0535. A lazy `map` steps from its back, its source as it is

Status: Accepted. Extends [0164](0164-double-ended-iterators.md),
[0429](0429-a-double-ended-iterator-knows-its-back.md) and
[0455](0455-stepping-from-the-back.md).

Case: C ([0262](0262-when-rust-and-js-disagree.md)): the items, their order,
and when the function runs are Rust's. A wrong result before.

## Context

A range mapped by a function generic code is given, not a closure made
there, folded from its back, itertools' `PadUsing::rfold`:

```rust
(self.iter.len()..self.min).map(self.filler).rfold(init, &mut f)
```

gave nothing. Its `map` had made the range a JS iterator,
`$iterator(range, $rangeNext)`, and stepping it from its back made that a
JS iterator again, stepping the JS iterator by `$rangeNext` as if it were
the range. And `rev()` of such a map was refused, "`rev` of a lazy
iterator", though it steps from its back as `$mapEnds`, which runs its
function as Rust's `next_back` does (ADR 0455).

## Decision

**A JS iterator `stepped_ends` made is stepped from its back from what it
steps**: `iterators::stepped_value` reads `$iterator(value, next, ..)`'s
`value`, the one reader of that form, beside the one writer, and
`as_double_ended` steps that.

**`rev()` of a lazy iterator, which rustc found double-ended, is `$rev` of
it stepped from its back**, `as_double_ended`'s: a map's `$mapEnds`, a
filter's `$filterEnds`. One that can't step from its back is refused
there, saying which, "a `Zip<..>` as a `DoubleEndedIterator`".

```js
const back = $rev(
  $mapEnds($iterator(range, $rangeNext, false, $rangeNextBack), (item) => f(item)),
);
```

## Why

`range_mapped_by_a_function` folds a range mapped by a function it's given
from its back, empty too, reverses one and steps it, and folds a type
parameter's from its back, against native Rust.

## Consequences

- itertools' `PadUsing` compiles.
- Three refusals say what can't step from its back now, not "lazy".
- Mutations step the JS iterator again, and refuse a lazy `rev()`.
- `len()` of a lazy `map`'s `rev()` is still refused: the map isn't made
  knowing its length, which the iterators draft's analysis answers.
