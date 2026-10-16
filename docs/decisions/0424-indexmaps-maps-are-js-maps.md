# 0424. indexmap's maps are JS's `Map` and `Set`

Status: Accepted. Extends [0423](0423-hashbrowns-maps-are-stds.md), whose
crates' maps are std's.

Case: A ([0262](0262-when-rust-and-js-disagree.md)) for what a map holds,
gives, and the order it gives it in, which are indexmap's; B for how long
what's done by a place takes, listed below.

## Context

indexmap's `IndexMap` and `IndexSet` keep what's added in the order it's
added, and find it by a key's hash as a `HashMap` does: serde_json's
`preserve_order` map, toml's, and many more crates' are one. Compiled as
it's written, it's a `Vec` of entries and hashbrown's `HashTable` of
indices into it, keyed by hashes std's `Hasher` makes, which rust-js
hasn't (ADR 0168). A JS `Map` keeps what's added in the order it's added,
and finds it by key: what an `IndexMap` is.

## Decision

**indexmap is read as rustc builds it, as hashbrown is, and its `IndexMap`,
`IndexSet` and map entries are std's maps: a `Map` and a `Set`. What it
does by an entry's place, which std's maps haven't, is the runtime's,
`PlaceOp`'s:**

| indexmap | JS |
|---|---|
| `get_index_of(k)` | `$indexOf(m, k)` |
| `get_index(i)`, `first()`, `last()` | `$atIndex(m, i)`, of `0`, of `m.size - 1` |
| `insert_full(k, v)` | `$insertFull(m, k, v)`; a set's, `$addFull(s, x)` |
| `swap_remove(k)`, and `remove(k)`, which is it | `$swapRemoveKey(m, k)`: the last in its place |
| `shift_remove(k)` | std's `remove`: `Map`'s `delete` keeps the order |
| `pop()` | `$popLast(m)` |
| `sort_keys()`, a set's `sort()` | `$sortKeys(m, cmp)` |

- **Which are indexmap's is `recognition::bound_map_item`'s**, by name, as
  its entries are its map's alone, and `is_index_map`'s; which method of a
  map, a set or an entry is which, std's or indexmap's, is
  `std_types::map::map_method`'s, out of the dispatcher, `classify_inherent`,
  which is the shorter for it.
- **A key's place is where it's kept**: of a map keyed by value, its
  value's string (ADR 0121), `$keptAs`.
- What indexmap has besides is refused where it's called, as hashbrown's
  own is.

## Why

- **It's the program Rust runs**: `test/bound-maps.test.ts` runs
  `scripts/crate-corpus/indexmap.rs`, built in order, by entry, by place,
  removed by shifting and by swapping, `remove` and a map keyed by tuples
  among them, popped, sorted, and a set, against native indexmap.
- **It's what a person writes**: a `Map`, which keeps the order an
  `IndexMap` keeps.

## Consequences

- **What's done by a place walks the map**: `get_index`, `swap_remove`
  and `pop` take as long as the map is, where indexmap's are constant,
  and `shift_remove` is constant, where indexmap's walks what's after it.
  The answers are the same (case B).
