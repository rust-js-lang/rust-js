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

## Amendment: a copied default's own where clause gives its impl slots (#798)

A trait's default copied into an impl (ADR 0049) is given no drop of its
own: the impl's dictionary is, and the default reads the impl's. An
associated type only the default's own where clause names, `<Self::Item as
IntoIterator>::IntoIter` of a `Self::Item: IntoIterator`, was no slot of
the impl's, whose bounds alone `drop_items` read, so its values, the
`iter` of a `Held<<Self::Item as IntoIterator>::IntoIter>`, were never
dropped: a wrong answer (case C of ADR 0262).

- **An impl's slots are read of its bounds and, after them, of each
  default it keeps' own where clause in its terms**, `<T as
  Iterator>::Item: IntoIterator` (`slot_bounds`): one derivation, where
  each holds, so the impl's caller gives that drop as it gives any slot's.
  A clause naming the default's own parameters, which its caller gives
  (ADR 0163), isn't the impl's.
- **An associated type of a type its trait isn't implemented for**,
  `<u8 as IntoIterator>::IntoIter`, the impl's slot where its caller's
  arguments can't meet that default's where clause, as `(0..3u8)`'s, is no
  type: it drops nothing.
- A library's default its consumer's impl keeps is the library's function
  (ADR 0100), whose own slots were already given; a std trait's defaults,
  std's own, add none, as their clauses don't name the impl's slots.

```js
function tHolding(TIterator, dropT, dropTItem, dropTItemItem, dropTItemIntoIter) { ... }

$dropItems(all, (item) => {
  try {
    heldDrop_drop(item, dropTItemIntoIter);
  } finally {
    dropTItemIntoIter?.(item.iter);
  }
});
```

Held by `test/corpus/default_where_clause_item_drops.rs`, the
`default_where_clause_drops` crate pair, and
`in_place_collect_of_a_built_type_relayed.rs`'s default, whose items now
hold their iterators plainly, with native Rust. Still open: a non-blanket
impl's `Self::Item` in its terms (#803), a default's own type
parameters' drops (#802), and one numbered as an impl's (#804).
