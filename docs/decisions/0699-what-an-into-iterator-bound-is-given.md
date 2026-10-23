# 0699. What an `IntoIterator` bound is given is its items, as JS steps them

Status: Accepted. Amends [0061](0061-generic-iterators.md), whose
`IntoIterator` on a type parameter is given the JS iterator itself, and
[0528](0528-fuse-and-enumerate-from-either-end.md), whose
`iterable_items` refused a `&mut` range.

Case ([0262](0262-when-rust-and-js-disagree.md)): R, results change:
each threw, or gave another order.

## Context

A generic function bounded by `IntoIterator` iterates what it's given as
JS does, `Iterator.from(i)`. A caller gave it the value as it was, so:

- `bump(&mut v)` of an `I: IntoIterator<Item = &mut u32>` was given the
  array, not a handle on each number, and threw writing one;
- `total(&option)` was given the bare value, `Iterator.from(7)`, which
  throws;
- `firsts(&btree)` was given its JS `Map`, which steps in the order its
  keys were put in, not theirs.

`iterable_items`, "what a function takes as an `IntoIterator`" (ADR 0528),
answered each, but only `extend`, `chain`, `zip` and `entries` asked it.
It refused a `&mut` range, which ADR 0698 now steps where it is.

## Decision

- **A type parameter of the callee's bounded by `IntoIterator`, given no
  dictionary of it (its `bounds`, the one answer to that), is given
  `iterable_items` of its argument** (`given_iterators`): an `Option`'s
  items, a B-tree's in order, and a `&mut` collection's handles.
- **`ref_into_iter`, what std's `IntoIterator` of a `&C` or a `&mut C`
  gives in JS**, is the one answer of the dictionary generic code is given
  (ADR 0457) and of `iterable_items`: a handle on each item of a `&mut`,
  as `for x in &mut v` makes, std's own of a `&`. Not of a `&mut` to a type
  parameter, an iterator its dictionary steps.
- **A `&mut` range is stepped where it is** in `iterable_items` too, as
  `as_js_iterator` does: `v.extend(&mut r)` leaves `r` where Rust does.
- **What's written out folds**: `undefined == null` is `true`, and a
  comparison `&& true` is itself, `&& false` `false` where it does
  nothing: an `Option` known `None` where it's made, `count(None)`, is
  `count([])`, and its dead branches go (five snapshots).

## Consequences

- Held by `test/corpus/iterables_given_to_generics.rs` against native
  Rust, and the mutations `into-iter-arg-raw`,
  `into-iter-arg-of-dictionary`, `mut-collection-items-raw`,
  `mut-param-made-items`, `into-iter-items-unhandled`,
  `mut-range-extend-refused`, `none-test-unfolded` and
  `compared-and-bool-unfolded`.
