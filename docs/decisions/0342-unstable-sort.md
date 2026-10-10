# 0342. An unstable sort is std's own, where its ties can be told apart

Status: Accepted. Amends [0036](0036-iterators-and-sorting.md); extends
[0335](0335-slice-views.md); counted by [0314](0314-std-data-structures.md).

Case: A ([0262](0262-when-rust-and-js-disagree.md)).

## Context

`sort_unstable`, `sort_unstable_by` and `sort_unstable_by_key` were JS's
`sort`, which is stable. Rust's leaves items that compare equal where its
own algorithm, `ipnsort`, leaves them, and from 21 items on that isn't
where a stable sort does: `sort_unstable_by_key(|p| p.0)` of 33 pairs
printed another order than native Rust, with no error. `select_nth_unstable`
and its `_by` and `_by_key` were errors.

## Decision

**Where items that compare equal can be told apart, an unstable sort and
`select_nth_unstable` are std's algorithms, step for step: `$sortUnstable`
and `$selectNthUnstable`, given std's `is_less` and what std picks by the
item's type.**

```rust
by_key.sort_unstable_by_key(|p| p.0);
lens.sort_unstable_by(|a, b| b.cmp(a));
*numbers.select_nth_unstable(middle).1 += 100;
```

```js
$sortUnstable(byKey, (a, b) => key(a) < key(b), "network");
$sortUnstable(lens, (a, b) => $cmp(b, a) < 0, "network");
const cell = $selectNthUnstable(numbers, middle, (a, b) => a < b, "network", true)[1];
```

| Rust | JS |
|---|---|
| `sort_unstable()` of numbers, text, `bool`s, `char`s and tuples and arrays of them | `v.sort(..)`, as `sort()` is: equal ones are the same |
| `sort_unstable()` of anything else, `_by(f)`, `_by_key(k)` | `$sortUnstable(v, isLess, kind)` |
| `select_nth_unstable(i)`, `_by`, `_by_key` | `$selectNthUnstable(v, i, isLess, kind)`: views of those before and after, and the item, a handle on a number or text |

- **Step for step**: insertion sort up to 20 items; then an existing run,
  kept or reversed; then quicksort, with std's pseudo-median pivot, its
  ancestor pivot, Lomuto's cyclic partition or, for items of more than 96
  bytes, Hoare's, its small sorts and heapsort when its recursion runs
  out. `select_nth_unstable` is std's introselect, with the median of
  medians when it runs out. Each asks `is_less` what std asks, in std's
  order, so a comparator that counts or prints sees what Rust's sees.
- **`kind` is what std picks by the type**: `"network"`, sorting networks,
  for a `Copy`, `Freeze` item of at most 8 bytes; `"general8"` and
  `"general"`, stable merges, for a `Freeze` one of at most 16 or 85;
  `"fallback"`, insertion sort; `"hoare"` for one of more than 96. Its size
  is rust-js's, of a 32-bit `usize` (ADR 0090).
- **`is_less` is std's**: `a < b` of numbers, else `cmp < 0`; a
  comparator's `compare(a, b) < 0`, of a closure of one expression in
  place, `(a, b) => $cmp(b, a) < 0`; a key's `key(a) < key(b)`.
- **Refused, loud**: an unstable sort of a type parameter's items, whose
  `kind` is its caller's type's.

## Why

- **It's the same program**: the same order, from the same comparisons.
- **It's the JS a person writes where it can be**: a sort whose ties can't
  be seen stays JS's `sort`.
- **It's tested**: the `sort_unstable_ties` corpus case runs, against
  native Rust, lengths from 10 to 300 of pairs by a key and by a
  comparator, a type whose own `Ord` ties distinct items, items of each
  `kind`, counting each comparison, items in order and in reverse, and
  `select_nth_unstable` of the middle, the first and the last, written
  through. `sort_unstable_adversary` runs McIlroy's adversary, which picks
  each comparison's answer to make every pivot the worst: std's sort falls
  back on heapsort, and introselect on the median of medians, and each
  ends where native Rust's does after as many comparisons. Mutations
  change each threshold, pivot, partition, network, merge and fallback.
- `docs/std-coverage.txt`: `slice` 125 of 133.
