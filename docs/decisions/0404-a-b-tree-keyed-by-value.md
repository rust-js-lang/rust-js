# 0404. A B-tree keyed by value, whose `Ord` is derived

Status: Accepted. Amends [0121](0121-value-keys.md).

Case: C ([0262](0262-when-rust-and-js-disagree.md)): a B-tree finds a
key by its `Ord`, so `$key`'s equality must be `Ord`'s, which a derived
`Ord` all through makes it; where it isn't, D: a compile error.

## Context

ADR 0121 made a map keyed by a tuple or a struct a `$KeyMap`, but refused
a `BTreeMap` or `BTreeSet` of those keys, as a string of the key's fields
isn't in the key's order. A B-tree is now a map in insertion order, sorted
by its keys' `cmp_fn` where its order is read (`$sortedEntries`), so its
storage needn't be in order: only its equality must be its `Ord`'s.
`BTreeMap<(i32, i32), _>`, a grid by its cells, was refused.

## Decision

**A B-tree keyed by value is a `$KeyMap` or `$KeySet`, read in its keys'
order, where the key's `Ord` is derived, or std's, all through:
`orders_by_value`'s answer. Two keys it orders `Equal` are then equal
field by field, as `$key` tells them.**

```js
const grid = new $KeyMap();
grid.set([2, 1], "a");
$sortedEntries(grid, (a, b) => $cmp(a[0], b[0]) || $cmp(a[1], b[1]));
```

- **A hand-written `Ord` anywhere in the key**, a struct's, or a
  fieldless enum's inside a tuple, is refused: it may order `Equal` keys
  that `$key` tells apart.

## Why

- **It's the program Rust runs**: the corpus's `value_key_trees.rs`
  keys B-trees by tuples, by a struct and an enum with fields that derive
  `Ord`, and by a tuple of an `Option` and a `Vec`, and inserts, gets,
  ranges, reads its ends, enters, removes, pops and retains, each with
  native Rust; the diagnostics test refuses two hand-written `Ord`s.
