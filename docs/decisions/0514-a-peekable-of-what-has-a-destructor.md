# 0514. A `Peekable` of what has a destructor

Status: Superseded in part by [0557](0557-std-bodies.md): a `Peekable` is
std's own struct, dropped as a struct's fields are, its source and what it
peeked at, with no `$peekable` to close. Extends [0164](0164-double-ended-iterators.md),
[0422](0422-an-iterator-holding-a-value-is-closed-to-drop-it.md) and
[0465](0465-dropping-an-adapter-is-closing-it.md).

Case: C ([0262](0262-when-rust-and-js-disagree.md)): what drops, and in
what order, would change, so Rust's is kept.

## Context

`v.into_iter().peekable()` of items with a destructor was refused, and so
were its `peek` and `next_if`. A `Peekable` of an array's items is the
array's `$iter`, which can't tell whether it peeked: Rust drops a
`Peekable`'s source first, then what it peeked at, `[1, 2, 3]` peeked at
`1` dropping `2, 3, 1`.

Finding it found an older bug: an adapter a generic function made of a
type parameter given as it is, `items.take(2)`, closed, gave its drop the
JS iterator stepping the value, `$iterator(items, next)`, not the value:
`drop counter undefined`.

## Decision

**`peekable()` of a source or items with a destructor is
`$peekable($closing(source, drop), itemDrop)`**: its source closed to drop
what it holds, and its own `return()` closing that first, then dropping
what it peeked at. Its `len()` and `next_back()` count and give what it
peeked at, as Rust's do, and `$closing` passes its source's `len` and
`back` through. `peekable_closes` is the one answer to which `Peekable` is
this, asked where it's made and where its length is
(`knows_its_length`). `peek`, `peek_mut`, `next_if` and `next_if_eq` drop
nothing.

**Not of a source that drops what it is, the crate's iterator's or a type
parameter's, `drops_what_it_is`**: stepped to its end by a JS iterator's
helper, it's never closed, where Rust drops it after; so that stays
refused, as before.

And a type parameter given as it is, closed, is dropped as its value,
`it.of`, as the crate's own iterator is.

## Why

- **It's the same program**: `peekable_drops` drops peeked and unpeeked
  `Peekable`s of an array's items and of a lazy source, counts and steps
  one from its back; `generic_adapter_source_drops` closes a generic
  `take`; both against native Rust.

## Consequences

- An adapter of a source that drops what it is, stepped to its end by a
  consumer, `Counter(0).map(f).sum()`, never drops that source: a JS
  iterator's helper, done, doesn't pass `return()` on. A gap of its own.
- Mutations drop what it peeked at first, or never, count and step from
  the back without it, take `peekable()` for its `$iter`, allow a source
  that drops what it is, and drop the JS iterator for the value.
