# 0339. `get_disjoint_mut`: a `&mut` to each, checked as std checks them

Status: Accepted. Extends [0335](0335-slice-views.md) and
[0152](0152-std-item-handles.md); counted by
[0314](0314-std-data-structures.md).

Case: A ([0262](0262-when-rust-and-js-disagree.md)).

## Context

`get_disjoint_mut([i, j])` gives a `&mut` to each of several items, or
ranges, of a slice at once, where none overlaps another; it and
`get_disjoint_unchecked_mut` were errors.

## Decision

**`$getDisjointMut(v, indices)`: `Ok` of each index's item, a handle on a
number or text, or each range's view (ADR 0335); or `Err` of std's
`GetDisjointMutError`, checked in std's order.**

```rust
if let Ok([a, b]) = v.get_disjoint_mut([0, 5]) {
    std::mem::swap(a, b);
}
```

```js
const value = $getDisjointMut(v, [0, 5], true);
if (value.TAG === "Ok") {
  const t = value._0[0].value;
  value._0[0].value = value._0[1].value;
  value._0[1].value = t;
}
```

| Rust | JS |
|---|---|
| `v.get_disjoint_mut([i, j])` | `$getDisjointMut(v, [i, j])`, with `true` where the items are numbers or text |
| `v.get_disjoint_mut([a..b, c..d])`, `[a..=b, ..]` | the same, of ranges, `a..=b` with `true` after |
| `get_disjoint_unchecked_mut(indices)` | its `_0` |
| `GetDisjointMutError::IndexOutOfBounds` | `"IndexOutOfBounds"`, as a fieldless enum is (ADR 0013): its derived `{:?}` and `==` |

- **In std's order**: each index in bounds, then apart from each before
  it, so `[9, 2, 2]` is out of bounds before it overlaps.
- **Each is a `[start, end)`**: an index `i` is `[i, i + 1)`, `a..=b` is
  `[a, b + 1)` and in bounds only where `a <= b`.
- **Still refused**: the error's own `{}`, which is std's message; its
  `{:?}` and `==` are derived.

## Why

- **It's the same program**: each `&mut` writes the item Rust's does, and
  each `Err` is the one Rust gives.
- **It's tested**: the `slice_disjoint` corpus case runs, against native
  Rust, two items swapped, out of bounds, overlapping, both at once,
  `==` of the error, ranges written through, overlapping and out of
  bounds, inclusive ones and a backwards one, the unchecked form, and
  structs. Mutations drop each check, the handle and the inclusive flag,
  and give the unchecked form the `Result`.
- `docs/std-coverage.txt`: `slice` 115 of 133.
