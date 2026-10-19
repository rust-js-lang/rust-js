# 0473. `fuse()` is Rust's `Fuse`

Status: Accepted. Amends [0055](0055-iterator.md); extends
[0465](0465-dropping-an-adapter-is-closing-it.md) and
[0170](0170-size-hint.md).

Case: C ([0262](0262-when-rust-and-js-disagree.md)): what's stepped, what's
dropped and when, and what's shown, as std's; B for generic code's
`FusedIterator`, below.

## Context

ADR 0055 made `fuse()` the iterator itself: a JS iterator chain stays ended
after its first `done`, and only a direct `next()` on the `Fuse` could tell
one that starts again after its `None`, which was refused then. It isn't
now (ADR 0071): `Flip(0).fuse()` stepped three times gave `Some(1) None
Some(3)`, where Rust's gives `Some(1) None None`. And a `Fuse` held as a
value, a field of itertools' `Interleave`, has std's `size_hint`, `Clone`
and `Debug`, each refused. std's `Fuse` is `{ iter: Option<I> }`: of what
isn't a `FusedIterator`, it drops `iter` as it ends, and steps it never
again; a `FusedIterator`'s it keeps.

## Decision

**`it.fuse()` is `$fuse(it)`, std's `Fuse`**: its `iter` stepped till it
ends, then never again; of what isn't a `FusedIterator`, closed as it ends,
which drops it (ADR 0465), and `undefined` then; of a `FusedIterator`,
`$fuse(it, true)`, kept.

- **Whether it's a `FusedIterator` is rustc's of a type it knows whole**;
  generic code's, which may be any, and of a library its callers never
  give a fact (ADR 0145), is cleared as it ends, as most iterators'.
- **Stepped in place by what steps it till it ends**, a JS helper, a
  consumer or a `for`, it's what it steps, `x`, which steps the same: tidy
  writes no `$fuse` there, `iterators::consumed_fuses`.
- **`size_hint()`** is `$sizeHint`'s, what it steps, or `(0, Some(0))` once
  it's ended; **`Debug`** is `Fuse { iter: Some(..) }` or `Fuse { iter: None
  }`, of what it steps as its type has it, a `$iterator`'s value,
  `$iteratorValue`; **`Clone`** is a `$fuse` of a clone of that, stepped as
  it was, `$iteratorWith`, whose `$iterator` is made again by its `with`.
- **An iterator over an array shows the slice it has left**, as std's
  `Debug` does: `IntoIter([2, 3])`, `Iter([..])`, `Drain([..])`.
- **An adapter closes what it holds whatever its items**: a crate's
  iterator with a `Drop` of its own, given to `fuse()` or `map(f)`, is
  closed as the adapter ends or is dropped, as std drops it (ADR 0465).

```js
let f = $fuse($iterator([0], flipIterator_next));
const total = $closing($iterator(["summed", 4], loudIterator_next), drop).map((n) => Math.imul(n, 10));
```

## Why

- **It's std's**: `fused_iterators` steps a `Fuse` of an iterator that
  starts again, shows, clones and sizes one, of the crate's iterator and a
  `Vec`'s, in generic code too, and drops a `FusedIterator`'s and another's
  each when native Rust does.
- **One representation**: a `Fuse` made in generic code and read in
  concrete code is the same `$fuse`.

## Consequences

- **What compiles grows**: `size_hint`, `Clone` and `Debug` of a `Fuse`,
  and `Debug` of an iterator over an array.
- **What was wrong is right**: `fuse()` of an iterator that starts again,
  and a crate iterator's `Drop` under an adapter.
- **Case B, rare and harmless**: generic code's `fuse()` of what's a
  `FusedIterator` clears it as it ends, where std's keeps it: a `Debug` of
  the `Fuse` once it's ended shows `None`, not `Some` of the ended one, and
  a crate's `FusedIterator` with a `Drop` of its own is dropped as it ends,
  not with the `Fuse`. What it steps is std's either way.
