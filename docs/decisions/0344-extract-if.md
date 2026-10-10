# 0344. `extract_if` is a lazy JS iterator

Status: Accepted. Extends [0139](0139-lazy-chains.md) and
[0325](0325-map-and-set-methods.md); counted by
[0314](0314-std-data-structures.md).

Case: A ([0262](0262-when-rust-and-js-disagree.md)).

## Context

`extract_if` of a `Vec`, a `LinkedList`, a `HashMap`, a `HashSet`, a
`BTreeMap` and a `BTreeSet` gives an iterator that, as each item is asked
for, asks a filter of the next and takes out each it holds of. What it
doesn't reach stays: a `break` leaves the rest, and the filter, given a
`&mut`, may change what it keeps. Each was an error.

## Decision

**`extract_if` is a JS iterator, made by `Iterator.from`, as std's lazy
sources are (ADR 0139): each `next()` asks the filter of what's next and
takes out what it gives.**

```rust
for x in w.extract_if(2..8, |x| { *x *= 10; *x > 40 }) {
    if x > 60 { break; }
}
```

```js
for (const x of $extractIf(w, 2, 8, (x) => { x.value = Math.imul(x.value, 10); return x.value > 40; }, true)) {
  if (x > 60) break;
}
```

| Rust | JS |
|---|---|
| `v.extract_if(range, f)`, a list's `extract_if(f)` | `$extractIf(v, start, end, f)`, checked as `slice::range` checks as it's made |
| a map's or a set's `extract_if(f)`, a B-tree's `extract_if(range, f)` | `$mapExtractIf(m, f, entries, handles, set)`, of its entries as it orders them, a B-tree's in its range |

- **The filter is given a handle on a number or text**, as `retain_mut`'s
  is (ADR 0152), and a map's what's in the map once it's run.
- **A B-tree's range isn't checked**: std's starts at its start and stops
  past its end, so one that ends before it starts takes nothing.
- **Since**: a range collected into a set or a map, or extending one, is
  its items, `new Set($range(1, 7))`, where it was the range's object, a
  `TypeError` (ADR 0129).

## Why

- **It's the same program**: what's taken, what stays, and what the filter
  changed, as Rust's leaves them, a `break` too.
- **It's tested**: the `extract_if` corpus case runs, against native Rust,
  each collection's, a range of a `Vec` left at a `break` with what the
  filter changed, of nothing, `String`s, a map's values changed, a
  B-tree's range and one that ends before it starts, and one dropped after
  two; `extract_if_past_len` panics as it's made; `range_into_set` makes
  and extends sets of ranges. Mutations run past the range, leave it
  unchecked, the handles out, give stale values, keep what's given, check
  a B-tree's range, ignore a `Vec`'s, and refuse each.
- `docs/std-coverage.txt`: `Vec` 37 of 48, and each other's `extract_if`.
