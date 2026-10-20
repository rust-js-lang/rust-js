# 0491. `cycle()` keeps a clone of its iterator

Status: Accepted. Extends [0181](0181-iterator-clones.md), whose `$cycle`
goes over an array's items, and [0465](0465-dropping-an-adapter-is-closing-it.md).

Case: C ([0262](0262-when-rust-and-js-disagree.md)): each item, and each
clone and drop, as std's `Cycle` makes them.

## Context

`cycle()` was known only of std's iterator over an array, whose clone is
what it has left (ADR 0181). Of a range, the crate's own iterator or a
type parameter's it was refused, and of items with a destructor too, as
an adapter that might leave some undropped. itertools'
`circular_tuple_windows` cycles an `I: Clone` given as it is.

## Decision

**`cycle()` of any other iterator is `$cycleOf(it, clone, next, boxed,
drop)`, Rust's `Cycle`: a clone of it kept as it's given, and each time
what it steps ends, a fresh clone of that stepped instead.** Of a range,
`$rangeNext` and its clone; of the crate's own, its impls' `next` and
`clone`; of a type parameter given as it is, its dictionaries',
`IIterator.next` and `IClone.clone`.

- **Its items are each its own**: each is made by a step of a clone, so a
  `Cycle` leaves none undropped, `leaves_none`.
- **Of an iterator with a destructor**, given its drop, it drops the one
  it replaces once the next clone is made, as an assignment does, and,
  closed, both it holds, the kept clone first, as `Cycle`'s fields drop;
  a `Cycle` is closed on drop, `closed_on_drop`, as `repeat`'s is (ADR
  0422). It's made at once, not a generator, so one dropped unstepped
  drops what it was given.
- **An adapter closed to drop what it holds**, `take(n)`'s, made of a
  type parameter given its drop, is closed once it's consumed, as one of
  the crate's iterator is (ADR 0464).
- **`$cycle` of an array's items with a destructor is refused**: it would
  leave what it holds undropped.
- **A function called with one argument alone is that function**,
  `Expr::called_alone`: `IClone.clone`, not `(iterator) =>
  IClone.clone(iterator)`; not where it's given more, as a JS array's
  `map` gives an index.

```js
function firsts(iter, n, IIterator, IClone) {
  return $cycleOf(iter, IClone.clone, IIterator.next, true).take(n).toArray();
}
```

## Why

- **It's std's**: `cycled_iterators` cycles a range, an empty one, the
  crate's own, a vector's iterator and a type parameter's, items with a
  destructor, an iterator with one, stepped and unstepped, and takes from
  a type parameter given its drop, with native Rust.

## Consequences

- **What compiles grows**: `cycle()` of any `Clone` iterator rust-js
  steps, and itertools' `circular_tuple_windows`.
