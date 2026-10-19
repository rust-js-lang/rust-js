# 0478. An associated type of one is given its drop too

Status: Accepted. Extends [0421](0421-std-associated-type-drops-are-given.md)
and [0445](0445-extend-and-collect-in-generic-code.md).

Case: C ([0262](0262-when-rust-and-js-disagree.md)): what's dropped, as
std's.

## Context

A std trait's associated type of a type parameter is given a drop of its
caller, `dropIItem` of `I::Item` (ADR 0421). One of that, a bound names,
`<I::Item as IntoIterator>::Item` of an `I::Item: IntoIterator`, wasn't, and
its values were refused: itertools' `kmerge_by`, `concat` and
`multi_cartesian_product` hold them. And `a.extend(b)` of such an
`I::Item`, an `IntoIterator` too, gave its dictionary `a`, where std's
`Extend` dictionary reads its collection in a box.

## Decision

- **A bound on an associated type that has a drop slot gives its own
  associated types slots, in turn**: `drop_items` finds each, and the
  analysis gives each its drop where what it's of is given one.
- **`Extend`'s `&mut Self` of a type only its caller knows is given as its
  trait's method takes it**, in a box, as `Hash`'s hasher is (ADR 0463).

```js
function firsts(iterable, dropIItem, dropIItemItem) { ... }

self = { value: a };
IItemExtendIItemItem.extend(self, b, dropIItem, dropIItemIntoIter);
$assign(a, self.value);
```

## Why

- **It's std's**: `nested_item_drops` takes the first of each of a `Vec` of
  `Vec`s of values with a destructor, and concatenates them, through
  generic code, dropping each when native Rust does.

## Consequences

- **What compiles grows**: values of an associated type of an associated
  type, as itertools' `kmerge_by` and `concat` have.
