# 0325. Maps' and sets' own methods, a B-tree's in its keys' order

Status: Accepted. Extends [0059](0059-hashmap.md) and
[0121](0121-value-keys.md); counted by
[0314](0314-std-data-structures.md). Amended: an entry's `and_modify`
(below).

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

- **A B-tree's range of a pair of `Bound`s, `(Excluded(&2), Unbounded)`,
  is its range too** (2026-10-17), which was refused: `$treeRange` takes
  its start's and its end's `Bound`, as `start_bound()` and `end_bound()`
  give them, `{ TAG: "Included", _0: 2 }` or `"Unbounded"`, of a range
  of std's and of a pair alike, where it took whether each end was there
  and whether the end was included, which an excluded start has no place
  in. A start and an end that are equal and both excluded panic, as
  std's do. What a range's bounds are is `range_bound_values`'s, of a
  `RangeBounds` dictionary's too.

  ```js
  $treeRange(scores, $cmp, false, "BTreeMap", { TAG: "Included", _0: 2 }, { TAG: "Excluded", _0: 5 });
  ```

  The `btree_range_bounds` corpus case runs, against native Rust, each
  kind of `Bound` at each end, of a map, a set and `range_mut`, a pair
  given and one made where it's used, and `btree_range_excluded_panic`
  std's panic. rustc's `sync-send-iterators-in-libcollections.rs` passes.

## Amendment: an entry's `and_modify`

`m.entry(k).and_modify(f).or_insert(v)`, the way to count or update a
map's value, was refused. It's `$andModify(entry, f)`: `f` given the
value where the key is there, a handle on a number or text as `insert`
of a `VacantEntry` gives (ADR 0152), and the entry back. Before
`or_insert`, `or_insert_with` or `or_default`, it's a statement, and they
are of the map and the key, as of an entry alone:

```js
$andModify($entry(counts, c), (n) => {
  n.value = (n.value + 1) >>> 0;
}, true);
$orInsert(counts, c, 1);
```

A handle on a map's value nothing reads, of an `or_insert` whose `&mut`
goes unused, isn't made: `$mutGet` only makes a value of what it's
given, one of the runtime's pure helpers. `entry_and_modify` runs it
against native Rust, of a `HashMap` and a `BTreeMap`.

## Amendment: a set's operators

`&a - &b`, `&a | &b`, `&a & &b` and `&a ^ &b` of a `HashSet` or a
`BTreeSet` were refused as calls of `Sub::sub` and the like (#120,
#156). Each is its algebra's method, collected: `new Set($setAlgebra(a,
b, "difference"))`, a B-tree's in its order. Which operator a std trait
is, recognition asked by two lists of its own beside `value_operator`'s
and `assign_operator`'s; it asks those, and the architecture test holds
the one list. Held by `test/corpus/set_operators.rs`.

## Amendment: a range's bounds read its fields (#725)

A range's `start_bound()` and `end_bound()` read the bound out of an
object written in place, `{ start: 2, end: 5 }`, where it was one. MIR
holds a range in a local before it asks its bounds, `const value = {
start: 2, end: 5 }; value.start`, so it never was one. With
`range-bounds-literal-read` applied, the whole corpus and every other test
file print the same JS (the Nightly of 85f123290). So the branch and its
mutation are removed.

## Amendment: an `extract_if` drops what it holds where it's dropped (#945)

Case ([0262](0262-when-rust-and-js-disagree.md)): R. A key whose `Drop`
prints printed nothing where Rust prints it.

`m.extract_if(K(1)..K(4), f)` owns its range, and a map's or a set's
`ExtractIf { pred, inner: { range, .. } }` drops its test, then its range,
start then end, where it's dropped: after a collect or a `count` steps it
to its end, or where a variable that holds it is. The JS never dropped
either: `$treeRange` read the bounds, `$mapExtractIf` held nothing, and its
`Iterator.from` closed it as its first step was done.

**It's closed where it's dropped, and closing it drops what it holds**:
`closed_on_drop` says of a B-tree's `ExtractIf` its test then its range,
and of a hashed one's its test, each that has a destructor; the lowering
holds those in variables and gives `$mapExtractIf` their drop, `drop_each`'s
of the two in that order, which it runs once as it's closed (`$close`), a
`$Steps` now, one of `ITERATOR_HELPERS`, no `Iterator.from` around it. Four
snapshots read `$mapExtractIf(..)` where they read `Iterator.from(
$mapExtractIf(..))`. An owned `RangeInclusive` of a key with a destructor
is refused, as before, and so is `range` of owned bounds with one. Corpus
case `extract_if_drops_what_it_holds.rs`: owned `Range`, `RangeFrom` and
`RangeTo` bounds of a map's and a set's, of an order nothing sees and of
one that can be seen, collected, counted and held then dropped early, and
a hashed map's test that holds what drops; both builds. Mutations
`btree-extract-if-not-closed-on-drop`, `hash-extract-if-not-closed-on-drop`
(`lower/recognition/iterators.ts`), `extract-if-holds-nothing-to-drop`
(`lower/std_types/btree.ts`), `map-extract-if-closes-dropping-nothing`
(`runtime/extract_if.ts`); `scan-state-not-closed`,
`map-extract-taken-stale`, `map-extract-kept` and `set-extract-kept` name
their lines as they now read.
