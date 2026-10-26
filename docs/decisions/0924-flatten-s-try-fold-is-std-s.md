# 0924. A held `Flatten`'s `try_fold` is std's, a parameter an associated type equals is bound as it is, and a map's, a range's and a combinator's call is lowered of its values in one place

Status: Accepted, under the user's goal of 2026-10-26 ("keep going until
all done, the choice is your recommendation"). The plan "rust-js: one
representation per iterator", its second phase. Extends
[0899](0899-a-flatten-held-is-stds.md), [0106](0106-generic-traits.md)
and [0364](0364-mir-lowering.md).

Case: C ([0262](0262-when-rust-and-js-disagree.md)): results change.
With std's structs alone, a `skip_while` or an `any` over a `flat_map` of
`skip()`s asked each its source's `nth(2)` where Rust asks `nth(1)`: a
held flatten's `try_fold` wasn't std's, so it stepped by `next`. And a
`chain` of a B-tree, or of an `Option` or a `Result`, given as an
`IntoIterator`, crashed the compiler.

## Context

- std's `FlattenCompat::iter_try_fold` is of `impl<I, U> FlattenCompat<I,
  U> where I: Iterator<Item: IntoIterator<IntoIter = U>>`: `U` is bound
  by nothing but that equality, so rust-js didn't take it for an
  iterator, and a `&mut U`, `frontiter.insert(..)`'s, was a cell, where
  the nested `flatten`'s `&mut T::IntoIter` is the iterator (ADR 0071).
  std's own `Option::insert` of it was refused.
- What `std_values` lowers of a call's values alone is
  `lowered_from_values`'s catalog. A map's or a set's method, a range's
  and an `Option`'s or a `Result`'s combinator were out of it, each
  lowered of its values by its MIR handler instead, the same lowering in a
  second place, so a dictionary's `into_iter` of one, which lowers it
  through `std_values`, reached its `unreachable!`.

## Decision

- **A held `flatten()` and `flat_map()` compile std's `try_fold`**, so
  what a short-circuiting consumer asks of it is asked of each inner
  iterator as std asks it.
- **A type parameter the where-clauses equal to an associated type is
  bound as that type's declaration bounds it** (`declared_bounds`): `U`
  of `IntoIter = U` is an iterator, its `&mut` the iterator.
- **A map's or a set's method, a range's and a combinator's call is
  lowered of its values by `std_values`**, whoever asks: their MIR
  handlers are gone, but those lowered from their places, an entry's
  value, a B-tree's range, `extract_if`, and an `Option` of a place found
  given to a closure (ADR 0642).

## Consequences

- ADR 0908's `differs` is 20, from 23: `skip_try_fold.rs`,
  `btree_given_in_order.rs` and `flat_map_inner_iterables.rs` run as
  native Rust does with std's structs alone.
- `equated_iterator_parameter.rs`: a closure given a `&mut U` is given the
  iterator, `(inner) => $next(inner)`, where it was a cell,
  `(x0) => $next(x0.value)`.
- Mutations: `equated-bounds-unseen`, `values-range-call-refused`; and
  `values-comb-subject-dropped` and `values-map-call-refused`, which were
  `mir-comb-subject-dropped` and `mir-map-call-refused`.
