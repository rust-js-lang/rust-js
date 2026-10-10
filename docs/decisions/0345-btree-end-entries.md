# 0345. A B-tree's `first_entry` and `last_entry`

Status: Accepted. Extends [0325](0325-map-and-set-methods.md); counted by
[0314](0314-std-data-structures.md).

Case: A ([0262](0262-when-rust-and-js-disagree.md)).

## Context

A `BTreeMap`'s `first_entry()` and `last_entry()` give its least or
greatest key's `OccupiedEntry`, which reads, writes and takes out what's
there. Both were errors, and an `OccupiedEntry` had no form.

## Decision

**An `OccupiedEntry` is `[m, key]`, as `entry(key)`'s is; its methods are
the map's at its key.**

```rust
if let Some(mut first) = names.first_entry() {
    first.get_mut().push('!');
}
```

```js
const first = $endEntry(names, $cmp, false);
if (first) {
  const cell = $entryGet(first, true);
  cell.value += "!";
}
```

| Rust | JS |
|---|---|
| `first_entry()`, `last_entry()` | `$endEntry(m, cmp, last)`: `[m, key]` of its least or greatest key by its `cmp`, or `undefined` |
| `key()` | `e[1]` |
| `get()`, `get_mut()`, `into_mut()` | `$entryGet(e)`, a handle on a number or text (`true`) |
| `insert(v)`, `remove()`, `remove_entry()` | `$entryInsert(e, v)`, `$entryRemove(e)`, `$entryRemove(e, true)` |

- **Still refused**: matching an `Entry` as `Occupied` or `Vacant`, whose
  enum has no form yet.

## Why

- **It's the same program**: the same key, and each read and write of
  what's there.
- **It's tested**: the `btree_end_entries` corpus case runs, against
  native Rust, both of a map of `String`s and of numbers, `get_mut` and
  `into_mut` written through, `insert`, `remove`, `remove_entry`, and of
  an empty map. Mutations take the first for the last, copy for a handle,
  give what's put in, keep what's taken, and refuse each.
- `docs/std-coverage.txt`: `BTreeMap` 30 of 31.
