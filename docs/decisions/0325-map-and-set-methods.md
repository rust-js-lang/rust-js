# 0325. Maps' and sets' own methods, a B-tree's in its keys' order

Status: Accepted. Extends [0059](0059-hashmap.md) and
[0121](0121-value-keys.md); counted by
[0314](0314-std-data-structures.md).

Case: C ([0262](0262-when-rust-and-js-disagree.md)), B for a `HashMap`'s
order.

## Context

ADR 0314's count had `HashMap` at 15 of its 33 methods, `HashSet` 8 of
30, `BTreeMap` 14 of 31 and `BTreeSet` 7 of 27: even `clear`, `retain`,
`extend` and a set's `union` were refused, and so was every B-tree method
of its order, `first_key_value`, `pop_first`, `range`.

## Decision

**Each is std's, on the map's JS `Map` or `Set`, or its `$KeyMap` or
`$KeySet`, by a helper of `map_ops.js`; a B-tree's in its keys' order, by
its keys' `cmp`.**

| Rust | JS |
|---|---|
| `clear`, `reserve`, `shrink_to`, `shrink_to_fit` | `m.clear()`; nothing (ADR 0315) |
| `m.retain(f)`, `s.retain(f)`, `drain`, `extend(items)` | `$retainMap(m, f, entries, handles)`, `$retainSet`, `$drainAll`, `$extendMap` |
| `get_key_value`, `remove_entry`, `into_keys`, `into_values` | `$getKeyValue`, `$removeEntry`, its keys or values, a B-tree's sorted |
| a set's `get(x)`, `take(x)`, `replace(x)` | `s.has(x) ? x : undefined`, `s.delete(x) ? x : undefined`, `$setReplace` |
| `union`, `intersection`, `difference`, `symmetric_difference` | `$setAlgebra(a, b, op)`, a B-tree's sorted by `cmp` |
| `is_subset`, `is_superset`, `is_disjoint` | `$isSubset(a, b, superset)`, `$isDisjoint` |
| `first_key_value`, `last`, `pop_first`, `pop_last` | `$treeEnd(m, cmp, last, set, pop)` |
| `range(a..b)`, `split_off(&k)`, `append(&mut other)` | `$treeRange`, `$treeSplitOff`, `$treeAppend` |

- **`retain`'s `f` is given a handle on a value that's a number or a
  string** (ADR 0152), so what it writes is the map's, and visits a
  B-tree's entries in order.
- **A `HashMap`'s and a `HashSet`'s order is the JS map's**, the order
  things went in: Rust's is its hash order, which no program can count on.
- **A range's start past its end panics**, `range start is greater than
  range end in BTreeMap`, as std's does; bounds of `(Bound, Bound)` are
  refused.
- **Refused, loud**: a set's item that could look like `None` given back,
  `first_entry` and `last_entry`, `extract_if`, a hasher's methods, and a
  map of values with a destructor's `clear`, `retain` and `extend` (ADR
  0321).

## Why

- **It's the same program**: each keeps, takes and orders entries as
  std's does.
- **It's tested**: a corpus case runs each against native Rust, `retain`
  writing a number through its handle, set algebra of hash and B-tree
  sets, a B-tree's ends, ranges with an excluded end that's there,
  `split_off` at a key, and `append`; another the reversed range's panic.
  Mutations keep what `retain` rejects, reverse a B-tree's ends, include a
  range's end, skip the panic, split past the key, leave `append`'s
  source, unsort a union, ask `is_subset` for `is_superset`, give `retain`
  no handle, and keep what `take` takes.
- `docs/std-coverage.txt`: `HashMap` 25 of 33, `HashSet` 24 of 30,
  `BTreeMap` 27 of 31, `BTreeSet` 26 of 27.

## Since

- **A B-tree map's `range_mut` is its `range`, each value of numbers or
  text a handle on it, as `iter_mut`'s is** (2026-10-10), and a `for` takes
  a B-tree's range as the array it is, which it refused.

  ```rust
  for (k, v) in scores.range_mut(2..5) {
      *v += *k as i32;
  }
  ```

  ```js
  for (const item of $mutEntries(
    scores,
    $treeRange(scores, $cmp, false, "BTreeMap", true, 2, true, 5, false),
  )) {
    item[1].value = (item[1].value + (item[0] | 0)) | 0;
  }
  ```

  The `btree_range_mut` corpus case runs, against native Rust, numbers
  and strings written in a `for`, a `map`, `last()` and a stepped
  iterator, tuples written in place, an empty range, and a `for` over a
  set's range. Mutations give numbers for handles and handles for tuples,
  and refuse each. `docs/std-coverage.txt`: `BTreeMap` 31 of 31.
