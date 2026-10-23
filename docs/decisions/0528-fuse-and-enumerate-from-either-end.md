# 0528. A `fuse` and an `enumerate` from either end, and what an `IntoIterator` gives

Status: Accepted; superseded in part by [0566](0566-fuse-is-stds.md), whose
`Fuse` is std's own, its back and its length its compiled impls'. Extends
[0455](0455-stepping-from-the-back.md),
[0456](0456-a-generic-exact-size-iterator.md) and
[0473](0473-fuse-is-rusts-fuse.md); accepts part of the
[iterators draft](draft/cloned-and-buffering-iterators.md).

Case: C ([0262](0262-when-rust-and-js-disagree.md)): the items, their
indices and their order are Rust's.

## Context

Rust's `Fuse` is double-ended and exact-sized where its source is, and its
`Enumerate` is exact-sized where its source is, and double-ended where its
source is both. rust-js's `$fuse` kept neither, and an `enumerate` was a
JS helper's `map((x, i) => [i, x])`, which has no back: generic code's
`it.fuse().len()`, `it.enumerate().rev()` were refused, four of itertools'
refusals among them.

Writing their test found worse: what `extend`, `chain`, `zip` and a
`Debug` builder's `entries` take, any `IntoIterator`, was passed as it is.
An `Option`'s `Some(4)` was `Array.from(4)`, nothing; a range given to
`chain` was spread as an object, `[...{ start: 5, end: 7 }]`, which throws.
Two functions answered the question in part: `iter_source`, of the crate's
own iterator, and `extend`'s own match of a range.

## Decision

**A `fuse` knows its back and its length where its source does; an
`enumerate` knows its length where its source does, and its back where
its source knows both, `$enumerateEnds(it)`, whose index from the back is
where the item is, the count so far and what was left, as Rust's
`next_back` counts it.**

- One answer to which adapters keep their source's length,
  `length_kept_source` (`Map`, `Copied`, `Fuse`, `Enumerate`), beside
  `back_stepped_source`'s (ADR 0526), now with `Fuse`.
- `$enumerateEnds` is a JS iterator as it's made, in `ITERATOR_HELPERS`.
  (Amended: `$enumerate`, which keeps its source and count too,
  [ADR 0542](0542-a-held-adapter-keeps-its-source-and-state.md).)

**What a function takes as an `IntoIterator` is its items, as JS steps
them: `iterable_items`, the one answer.** An `Option`'s are `o == null ?
[] : [o]`; a range's, the crate's own iterator's or a type parameter's
given as it is, its JS iterator; an array's or a JS iterator's, itself; a
`&mut` range, which it would step, refused. `extend`, `chain`, `zip` and
`entries` ask it; `iter_source` is gone. An `Option`'s items are an array
made here, not copied again (`fresh`).

## Why

`fuse_enumerate_ends` steps generic code's `enumerate` from its back and
both ends, counts what's left, and steps a `fuse` from both ends and counts
it; `into_iter_arguments` extends, chains, zips and lists as `Debug` entries
`Some`, `None` and ranges, against native Rust.

## Consequences

- Ten snapshots read better: `(o == null ? [] : [o]).length`, `[0]`, no
  `Iterator.from` or `Array.from` around an array made there.
- Mutations drop a `fuse`'s back or length, count from the back one past,
  stop seeing an `enumerate`'s length or back, make it a JS helper, and
  take an `Option` or a range as it is.

## Amendment: an `enumerate` of an array's items, held or given, from its back

`it.enumerate().rev()` of a slice's iterator a variable held, or a
function of the crate's gave, was refused: an `enumerate` of an array's
items is `src.map((x, i) => [i, x])`, which has no back, and only one
made in place was reversed, as an array. And `rev()` of such an iterator
a function gave threw: it was taken for the array, where it's its `$iter`
or the array.

- **`as_double_ended` makes one `$enumerate` of their `$iter`**, which
  knows its back and its length, where it meets the form
  (`iterators::enumeration_source`, the one reader of `enumerated`'s).
  One made in place stays the array's `map`, reversed.
- **`rev()` of what a function gives is what's left of it**, `$rest(it)`,
  as of one a variable holds: either its `$iter` or its array
  (`given_by_a_call`).
- A held `enumerate` stepped by `next_back` stays refused.

ADR 0262's case: C, an error where Rust is fine, and a throw. Held by
`test/corpus/returned_slice_iters.rs` and the mutations
`rev-of-returned-iter-unwrapped`, `enumerate-of-array-no-back` and
`enumeration-source-unread`.
