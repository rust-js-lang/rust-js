# 0467. A map's keys with a destructor drop as Rust's do

Status: Accepted. Amends [0321](0321-map-drops.md), which refused them.

Case: C ([0262](0262-when-rust-and-js-disagree.md)): each key dropped once,
where Rust drops it; a `HashMap`'s in the map's order, as ADR 0321's values.

## Context

A map or a set keyed by a value with a destructor was refused (ADR 0321):
`insert` of a key equal to one there keeps the old key and drops the new
one, which a JS `Map` doesn't. A library's generic code keys a map by its
consumer's type, which may have one, so itertools' `counts`, `unique` and
`duplicates`, a trait's defaults keyed by `Self::Item`, were refused as a
library's (ADR 0466).

## Decision

**A map's and a set's keys drop as Rust's do: each operation that drops one
is given the keys' drop, last, as ADR 0321's are given the values'.**

```js
$insert(m, [1], 11, keyDrop_drop);       // drops the key given, where one's there
$remove(m, [2], keyDrop_drop);           // drops the key it held
$orInsert(counts, item, 0, undefined, dropTItem);
for (const [key, value] of m) { keyDrop_drop(key); loudDrop_drop(value); }
```

- **The key given, where an equal one's there, which the map keeps**:
  `insert`'s, `entry`'s, `or_insert`'s and `or_insert_with`'s, `extend`'s,
  and a set's `insert`'s.
- **The key it held**: `remove`'s, a set's `remove`'s, and those `retain`
  doesn't keep, each with its value, key first. `remove_entry` gives back
  the key it held, `$storedKey`'s, not the one it's asked with.
- **Each key, then its value, as Rust drops a pair**, as it's cleared or
  dropped, a B-tree's in its keys' order, a set's items.
- **A copied default is given each drop of an associated type its impl's
  given**, `dropTItem` of `T::Item`, found of the trait's terms,
  `Self::Item`, too.

## Why

- **It's Rust's, drop for drop**: `map_key_drops` inserts, enters, removes,
  extends, retains, clears and drops keys with a destructor, of `HashMap`,
  `HashSet`, `BTreeMap` and `BTreeSet`, with native Rust, one entry left
  where a hash order would show; `item_keyed_map` counts a consumer's keys
  with one in a trait's default, as an app and as a library.

## Consequences

- **An entry and a set's `replace` are of the key given**, equal to the one
  the map holds: only their addresses tell them apart.

## Amendment: a B-tree drops the key given before it writes the value (2026-11-08, #921)

Case C ([0262](0262-when-rust-and-js-disagree.md)): results change, so
Rust's behaviour is kept.

`$insert` and `$extendMap`'s put wrote the new value, then dropped the key
given, then the old value. That is hashbrown's order: its `insert` replaces
the value and drops `k` at its end. A `BTreeMap`'s `insert` is `entry(key)`,
which drops the key given as it finds an equal one, before the value is
written. Where that drop panics, the value given is `insert`'s local,
dropped as it unwinds, and the map keeps its old value. rust-js left the new
value in the map and leaked the old one.

**A B-tree's `insert` and `extend` of a key it holds drop the key given
first, the value given dropped as that unwinds, then write the value**:
`$insert(m, k, v, dropKey, dropValue, true)` and `$extendMap(m, items,
false, drop, dropKey, true)`, `map::key_dropped_first`'s answer. Only where
the key's drop may panic (`drop_may_panic`), the one way the order is
seen; elsewhere the call is as it was. A hashed map's order is unchanged.

Held by `test/corpus/btree_insert_drops_key_first.rs`, a B-tree's `insert`
and `extend` and a hashed map's `insert` against native Rust in both
builds, and the mutations `insert-tree-*`, `extend-tree-*`.
`entry(k).or_insert(v)` of an occupied entry, where std's `entry` drops the
key before `v` is even made, is #911's `entry` rows.
