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

## Amendment: a key whose `Ord` is the crate's own

A B-tree keyed by a struct whose `Ord` is hand-written was refused, as
`$key` may tell apart two keys its `Ord` finds `Equal` (#165). Case A
now: std's B-tree finds a key by its `cmp` alone, and so does the map.

**A B-tree of a key `orders_by_value` doesn't answer for is an
`$ordMap(cmp)` or `$ordSet(cmp)`, `found_by_cmp`'s answer: its entries
kept in `cmp`'s order, each found by a binary search of it.** A key it
finds `Equal` finds the one it holds, as std's does, whatever its fields.

```js
const releases = new ($ordSet(releaseOrd_cmp))();
$storedKey(releases, { major: 2, name: "" });
```

- **The class is made of `cmp`**, so a copy, `new m.constructor(m)`, and
  `split_off`'s new half are ordered by it too.
- **A generic B-tree is given its class by its caller**, as a hashed one
  is (ADR 0466), under its own fact, `TreeMap` or `TreeSet`: a `HashMap`
  of the same key finds it by its fields, a `BTreeMap` by its `cmp`.
- **What gives back a key gives the one the map holds**, `$storedKey`:
  `get_key_value`, a set's `get`, `take` and `replace`, and an occupied
  entry's `key` and `remove_entry`. This lifts ADR 0401's and 0402's
  refusals of those for paths and `Cow`s, which find others too.
- **Made of items, it keeps the last of equal ones** (`$treeFrom`), as
  std sorts what it's made of and keeps the last; a generic B-tree's
  key, which its caller's `cmp` may find `Equal` to another, does too.

Consequences, only where an `Ord` disagrees with its `Eq`, which std
calls a logic error whose results it doesn't specify: std's `from_iter`
keeps two keys `cmp` finds `Equal` that `==` doesn't, and rust-js keeps
one. Held by `test/corpus/btree_own_ord.rs` and `held_keys.rs`, each
with native Rust.

## Amendment: what holds a `$OrdMap`

Case: N ([0262](0262-when-rust-and-js-disagree.md)): what was decided,
held; nothing a program sees changes.

Three mutations held `$OrdMap`, each of finding and keeping a key. Four
more are each caught (an architecture review, 2026-10-25): a key put at
its end, not where its order puts it, by `btree_own_ord.rs`; and, by
`test/runtime-package.test.ts`'s "a map keyed by a struct, hashed or
ordered, reads as a Map to the JS given it", a missing key's removal
taking its neighbour, `size`, and `forEach`'s value before its key, which
JS calls and no Rust program does. A B-tree handed to JS iterates in key
order only where its key has an `Ord` of its own; any other is a `Map` in
insertion order, which Rust sorts as it steps it: #237 asks which the
boundary should be.

## Amendment: a B-tree is read in its keys' order by the JS it's given (#237)

A B-tree of keys a hashed map finds, a number's, a string's, or a
derived `Ord`'s, was a `Map` or a `$KeyMap` in the order it was added,
sorted only where Rust read it (`$sortedEntries`): `ranks()`'s `new
Map([[2, "silver"], [1, "gold"]])` gave a JS `for...of` 2 first, where a
key of its own `Ord`, an `$OrdMap`, gave 1. One type, two orders a JS
caller sees, and the guide's "iterated in key order" true of one. Case C
([0262](0262-when-rust-and-js-disagree.md)): the order a caller reads is
the type's, Rust's, so JS's must be it too.

**A B-tree of keys a hashed map finds is a `$btreeMap(cmp, Base)` or
`$btreeSet(cmp, Base)`, of the class a hashed map of them is, `Map`,
`$KeyMap` or another, read in `cmp`'s order by Rust and by JS alike;
`$BTreeMap` and `$BTreeSet` of keys `$cmp` orders** (`map::btree_class`,
the user's choice of a lazily sorted class). A key added past its last
is in place; one added before it sorts its entries again, in place,
before it's next read, so adding and finding stay a hashed map's, and
reading in order costs a sort once each change.

```js
const ranks = new $BTreeMap([[2, "silver"], [1, "gold"]]);
const grid = new ($btreeMap((a, b) => $cmp(a[0], b[0]) || $cmp(a[1], b[1]), $KeyMap))();
```

- **Rust still reads it by `$sortedEntries`**, as a `Map` JS hands a
  `&BTreeMap` may be in any order: a sort of one already sorted is a
  pass over it.
- **A key of its own `Ord`** is an `$OrdMap` as before, found by it.

Held by `test/runtime-package.test.ts` ("a map keyed by a struct, hashed
or ordered, reads as a Map to the JS given it"), each kind of key read by
JS's `for...of`, `keys()` and `forEach` in key order, one emptied before
it's sorted again read as empty, 64 corpus cases against native Rust,
and the mutations `btree-*` (`runtime/btree.ts`,
`lower/std_types/map.ts`).
