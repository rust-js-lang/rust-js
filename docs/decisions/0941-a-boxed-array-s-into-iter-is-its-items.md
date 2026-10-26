# 0941. A `Box<[T; N]>`'s `into_iter()`, 1.99's `BoxedArrayIntoIter`, steps the array's items it owns

Status: Accepted, under the user's goal of 2026-10-26 ("keep going until
all done, the choice is your recommendation"). The plan "rust-js: one
representation per iterator", its second phase. Extends
[0388](0388-a-boxed-slice-iterates-as-its-array.md).

Case: C ([0262](0262-when-rust-and-js-disagree.md)): Rust's behavior is
kept where rust-js refused it. With std's structs alone,
`Box::new([1u8, 2]).into_iter().rev()` was refused, "a
`BoxedArrayIntoIter<u8, 2>` as a `DoubleEndedIterator`".

## Context

A `Box<[T; N]>` is its array (ADR 0388), and 1.99's `into_iter()` of one,
`BoxedArrayIntoIter`, the array it was given (`Std::Pointee`). What steps
an array's items, owning them or not, is `array_source`'s, which named
`vec::IntoIter` and `array::IntoIter` but not it: so its back, its length
and what a `map()` of it made in place copied weren't an array's.

## Decision

- **`BoxedArrayIntoIter` is an iterator that owns an array's items**
  (`array_source`), as a `Vec`'s `into_iter()` is.

## Consequences

- ADR 0908's `differs` is 4, from 5: `boxed_array_iter.rs` runs as native
  Rust does with std's structs alone.
- `boxed_array_iter.js`: `Box::new(names).into_iter().map(f).collect()` is
  `names.map(f)`, where it was `Array.from(names).map(f)`.
- Mutations: `boxed-array-into-iter-unknown`.
