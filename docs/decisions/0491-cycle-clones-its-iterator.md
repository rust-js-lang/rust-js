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

## Amendment: both fields dropped though the first's drop panics (2026-11-07, #883)

Case ([0262](0262-when-rust-and-js-disagree.md)): results change, so
Rust's behavior is preserved. A closed `Cycle` dropped the clone it keeps,
then what it steps, in two plain calls. Where the kept clone's drop
panicked, what it stepped was never dropped. Rust's drop glue goes on: it
drops `iter` as `orig`'s drop unwinds. The close now runs both by
`$inTurn`, the one answer to what a drop of many parts runs as one of them
throws (ADR 1092 S7). `cycle_drops_both_as_one_panics` holds it against
native Rust, and `cycle-close-stops-at-a-panic` puts the two calls back.

## Amendment: its `size_hint()` is std's of each form (#1179)

`Cycle::size_hint` is `(0, Some(0))` of a source whose hint is that,
`(0, None)` of another whose lower bound is none, else `(usize::MAX,
None)`. `$cycleOf` asked its source's hint without the step it's given,
so of a range's, whose hint only its `next` knows (ADR 0918), it gave
`(0, None)` whatever the range; `$cycle`, of an array's items, a
generator, carried none, `(0, None)` too; and `size_hint()` of a `cycle()`
of a `Vec`'s or a slice's items was refused (the Crater, itertools'
`test_zip_longest_size_hint`).

**Each form gives std's hint**: `$cycleOf` asks its source by its step,
`$sizeHint(orig, { next })`, and `$cycle` carries its own, none of no
items, else without end; `$cycle` is one of the runtime's iterator
helpers so, not its generators. And a `cycle()`, always a `$Steps` that
asks its source's hint, is asked its `size_hint()` where its source's hint
is known, an array's items' among them, as `decode_utf16`'s is
(`asks_its_sources_hint`).

Held by `test/corpus/cycle_size_hint.rs` against native Rust on wasm32,
whose `usize::MAX` is rust-js's, in both builds. Mutations
`cycle-hint-of-its-source-unasked`, `cycle-of-items-hintless`
(`runtime/cycle.ts`) and `cycle-hint-unasked`
(`lower/recognition/types.ts`).
