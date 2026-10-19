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
