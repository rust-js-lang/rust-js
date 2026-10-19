# 0471. A generic unstable sort is sorted as its caller's type picks

Status: Accepted. Amends [0342](0342-unstable-sort.md); extends
[0145](0145-type-facts.md) and [0466](0466-a-generic-maps-class-is-its-callers.md).

Case: A ([0262](0262-when-rust-and-js-disagree.md)): the same order, from
the same comparisons.

## Context

std's unstable sort picks its small sort, and its partition, by its items'
layout: whether they're `Freeze` and `Copy`, and their size (ADR 0342). Two
equal items that can be told apart end where that algorithm leaves them,
so rust-js runs std's own, `$sortUnstable(items, isLess, kind)`. Of a type
parameter's items, `v.sort_unstable()` in generic code, the kind wasn't
known, and it was refused. itertools sorts so in `sorted_unstable`,
`sorted_unstable_by`, `sorted_unstable_by_key` and `k_smallest`'s
`select_nth_unstable_by`.

## Decision

**What std's unstable sort picks for its items is a fact of their type,
`TypeFact::Sort`, which a caller of generic code gives**, as a map's class
is (ADR 0466): `TSort`, or `SelfItemSort` of a trait's default, its value
the kind its caller's type picks:

```js
function sorted(items, compare, TSort) {
  $sortUnstable(items, (a, b) => compare(a, b) < 0, TSort);
  return items;
}

const by = sorted(tags.slice(), by_key, "network");
```

- **Asked of an unstable sort's items**, a type parameter's or a std
  associated type's of one: `sort_unstable`, `select_nth_unstable` and
  their `_by` and `_by_key`, which `recognition::unstable_sort` names, for
  `classify` and the analysis alike.
- **Answered by `type_fact_value`**: of a type rustc knows the layout of,
  `sort_kind`; of a type parameter, its caller's.
- **Items that hold a type parameter, `(T, u8)`**, are still refused: their
  layout is their own, not one a caller gives.

## Why

- **It's std's order**: `generic_unstable_sorts` sorts items of a small
  `Copy` type and of a wide tuple, each a different kind, through a generic
  function, `select_nth_unstable`, and a trait's default, with native Rust.

## Consequences

- **What compiles grows**: an unstable sort of a type parameter's items.
