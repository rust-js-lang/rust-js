# 0926. What a call takes as an `IntoIterator` gives the `&mut`s its items are, so `zip(b)` of a `&mut [T]` steps a handle on each

Status: Accepted. Fixes #329. Extends
[0152](0152-std-item-handles.md) and [0364](0364-mir-lowering.md).

Case: C ([0262](0262-when-rust-and-js-disagree.md)): Rust's behavior is
kept where rust-js refused it. `for (ai, bi) in a.iter().zip(b) { *bi +=
*ai; }` with `b: &mut [u32]`, num-bigint's `__sub2rev`, was refused: "a
`&mut u32` from `std::iter::Iterator::zip` used as a value", where
`zip(b.iter_mut())` and `for bi in b` compiled.

## Context

A std call that makes a `&mut` to an item JS can't change in place is
given a handle on each, `$mutItems(..)` (ADR 0152); one whose `&mut`s an
argument gives already, an iterator of them, passes them on.
`makes_items_of` told the two apart by its arguments' types, `cells` of
each and of each iterator's item. `zip(b)`'s `b` is no iterator but an
`IntoIterator`, `&mut [u32]`, whose items, `&mut u32`, `zip` makes by its
`into_iter()`, which `iterable_items` gives a handle on each; so `zip`
looked like it made them itself, which nothing lowers.

## Decision

- **An argument a call takes as an `IntoIterator`, a type parameter its
  callee bounds by it, `zip`'s `U`, gives the `&mut`s its items are**:
  they're made where it's made into its iterator, each a handle.
- **Not an argument taken as itself**, `iter_mut`'s `&mut Vec<T>`, nor
  `into_iter`'s own `self`, the trait's `Self`: each call makes the items,
  `$mutItems(b)`, as before.

## Consequences

- `zip_into_mut_slice.rs` runs as native Rust does: a `&mut [T]`'s, a
  `&mut Vec<T>`'s and a `&mut [T; N]`'s items zipped and written.
- Mutations: `into-iterator-items-made`, `into-iterator-items-of-any-argument`,
  `into-iter-self-items-given`.
