# 0324. A slice's fills, copies, order checks, chunks and splits

Status: Accepted. Extends [0025](0025-vec-loops-refcell-mut.md) and
[0315](0315-vec-capacity-and-edits.md); counted by
[0314](0314-std-data-structures.md).

Case: C ([0262](0262-when-rust-and-js-disagree.md)), A where JS's own is
the same.

## Context

ADR 0314's count had slices at 37 of their 133 methods: `fill`,
`copy_from_slice`, `is_sorted`, `partition_point`, `chunks_exact`,
`rchunks`, `first_chunk`, a split by a predicate, `sort_by_cached_key` and
a byte slice's ASCII methods were refused.

## Decision

**Each is std's, on the slice's JS array in place where std's changes it,
by a helper of `slice_ops.js` where JS has no method of its own.**

| Rust | JS |
|---|---|
| `v.fill(x)` | `v.fill(x)`; `$fill(v, x, clone)` where a clone is more than the value: a clone in each but the last, `x` itself |
| `fill_with`, `copy_from_slice`, `clone_from_slice`, `swap_with_slice`, `copy_within` | `$fillWith`, `$copyFromSlice`, `$cloneFromSlice`, `$swapWithSlice`, `$copyWithin`, each panicking as std's does |

(Amended: `clone_from_slice` of items whose `Clone` has a `clone_from` of
the crate's calls it on each, as std's does, given a handle on a number or
text; of items with such a type inside them, or generic where the crate
has one, it's refused. It had cloned each, which a review found.)
| `is_sorted()`, `is_sorted_by(f)`, `is_sorted_by_key(f)` | `$isSortedBy(v, (a, b) => a <= b)` of numbers, `$cmp(a, b) <= 0` of others; `$isSortedByKey(v, f, cmp)` |
| `partition_point(p)` | `$partitionPoint(v, p)`, by std's binary search, `p`'s calls the same |
| `chunks_exact(n)`, `rchunks(n)`, `rchunks_exact(n)`, `.remainder()` | an array of chunks, and its `remainder` |
| `first_chunk::<N>()`, `last_chunk::<N>()` | `v.length >= N ? v.slice(0, N) : undefined` |
| `split(p)`, `splitn`, `rsplit`, `rsplitn`, `split_inclusive` of a predicate | `$sliceSplitBy(v, p, n, inclusive, back)` |
| `strip_prefix(p)`, `strip_suffix(p)`, `repeat(n)`, `sort_by_cached_key(f)` | `$sliceStrip`, `$repeatItems`, `$sortByCachedKey`, `f` once an item |
| a byte slice's `is_ascii`, `to_ascii_*case`, `make_ascii_*case`, `trim_ascii*` | `v.every((b) => b < 128)`, `$asciiBytes`, `$makeAsciiBytes`, `$trimAsciiBytes` |

- **`strip_prefix` and `strip_suffix` take items `==` compares by
  value**, as `starts_with` does: others stay refused.
- **A `&mut` to part of a slice is refused, loud**, as it was:
  `v[1..3].fill(0)` would change a copy of that part, not `v`.
- **`is_sorted_by_key` and `sort_by_cached_key` call the key function as
  std's do**: once an item, the first up to the first out of order.

## Why

- **It's the same program**: each changes, panics and calls its functions
  as std's does.
- **It's tested**: a corpus case runs each against native Rust, with
  objects cloned, counted calls, a NaN, chunks with what's left, and a
  split with an empty last piece; another `copy_from_slice`'s panic.
  Mutations share `fill`'s and `clone_from_slice`'s objects, skip the
  length check, stop `partition_point` short, keep a partial chunk, cut
  `rchunks` from the start, drop a split's empty end, call a cached key
  again, keep checking past disorder, and say a NaN is sorted.
- `docs/std-coverage.txt`: slices 67 of 133.
