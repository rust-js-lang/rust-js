# 0976. An `IntoIterator` parameter is held as it is, never in a box

Status: Accepted, under the user's goal of 2026-10-26. Amends
[0457](0457-an-into-iterator-bound-on-a-reference.md): its dictionary of a `&mut C`
takes a box only where generic code holds one.

Case ([0262](0262-when-rust-and-js-disagree.md)): results change. std's
compiled `zip` and `chain` threw where given a `&mut [T]` or a `&mut
Result`.

## Context

A `&mut` to an object in generic code is a box of it, `{ value }`, where
its type is unknown (ADR 0454): a `c: &mut C` of an unknown `C`. A
parameter bound by `Iterator` or `IntoIterator` is never one of those
(`is_generic_boxed`): it is given as a JS iterator, or held as it is.

Yet the `IntoIterator` dictionary of every `&mut C` took the box,
`(cell) => $iter($mutItems(cell.value))`. std's `zip<U: IntoIterator>`
given `b: &mut [u32]` holds `b` itself, so it read `cell.value` of the
array: `undefined is not iterable`. That was `zip_into_mut_slice` with
std's structs, and `result_given_as_items`'s chain of a `&mut Result`.

A function's own `impl IntoIterator` parameter, given on to std's `zip`,
had no dictionary at all.

## Decision

- The dictionary of a `&C` or a `&mut C`, `ref_into_iter_dictionary`,
  takes the box only where it's asked of how generic code holds one.
- A callee's bound on a type parameter, `U: IntoIterator`, given a
  reference, gets the dictionary that takes it as it is.
- A `&mut C` bound itself takes the box, as before.
- Of a function's own `IntoIterator` parameter, which it holds as the
  iterable its caller gave (ADR 0551), the dictionary's `into_iter` is
  the JS iterator of it, as a `for` steps it (`given_into_iter`).

## Consequences

`result_given_as_items` leaves the iterator forms' `differs`, and
`zip_into_mut_slice` and `associated_type_as_mut` run with std's structs.
The mutation `into-iter-param-boxed` checks this.
