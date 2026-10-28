# 0528. A `fuse` and an `enumerate` from either end, and what an `IntoIterator` gives

Status: Accepted; superseded in part by [0566](0566-fuse-is-stds.md), whose
`Fuse` is std's own, its back and its length its compiled impls'. Extends
[0455](0455-stepping-from-the-back.md),
[0456](0456-a-generic-exact-size-iterator.md) and
[0473](0473-fuse-is-rusts-fuse.md); accepts part of the
[iterators draft](draft/cloned-and-buffering-iterators.md).
Amended (#383): an `enumerate()` made in place of what isn't an array's
items, a `fuse()` of owned items with a destructor say, is `$enumerate`
of its source made knowing its back and length (`as_exact_size`), where
it's stepped from its back; `test/corpus/enumerate_of_a_fuse_from_back.rs`.

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

## Amended by

[ADR 0715](0715-what-an-into-iterator-bound-is-given.md): what a type parameter bounded by `IntoIterator` is given is `iterable_items` of its argument, a `&mut` collection's handles among them, and a `&mut` range is stepped where it is.

## Amendment: a closed source's length

`enumerate()` of an array's items with a destructor is `$enumerate` of
them closed, `$closing(..)`, which passed on its source's back but not
its length, so stepped from its back, `enumerate().map(..).rev()`, it
threw: `iter.len is not a function` (#211). `$closing` passes on its
source's length where it has one, as it does its back and its `nth`
(`enumerate_rev_drops`, `closing-length-unknown`). The case's two
iterators, closed to drop what they don't give, grow the readability
measure's `$iter` share, as any of items with a destructor do.

## Amendment: one of items with a destructor beside a held one (#383)

`vec![..].into_iter().fuse().enumerate().rev()`, of items with a
destructor, was refused where the same function held one of that type,
`let mut it = v.into_iter().fuse().enumerate(); it.next_back()`. The held
one drops if a later call panics, so those calls are in a `try`, and the
`rev()`'s enumeration, made before its call, is put in a variable, where
its form isn't seen.

- The back-stepped analysis records an enumeration of items with a
  destructor whose own `next_back()` the crate calls. `knows_its_back`
  reads it so, as it does a map or a filter so recorded (ADR 0545).
- The `rev()` then makes its source knowing its back where it's made,
  `backs.rs`' rule. Of std's compiled `Fuse`, that's `$knowing(fuse,
  ..)`, its compiled back and length, as `as_exact_size` makes it, not
  the struct as it is, which `$enumerate` would ask a `len()` of. Only
  of one so recorded: another's form, `$enumerate(iterator_fuse(..))`,
  stays as it was.

`test/corpus/enumerate_of_droppers_from_back.rs` checks both forms, their
indices and drop order, against native Rust. The mutations are
`enumerate-of-droppers-unrecorded`, `back-stepped-enumerate-unknown` and
`back-stepped-enumerate-fuse-unwrapped`.
