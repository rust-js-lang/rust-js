# 0557. A `Vec` into an array of its length

Status: Accepted. Extends [0189](0189-panic-display-slice-arrays.md).

Case: C ([0262](0262-when-rust-and-js-disagree.md)): `Ok` of the array
where the `Vec` is `N` long, else `Err` of the `Vec` given back, as
Rust's `TryFrom<Vec<T>> for [T; N]` gives.

## Context

aho-corasick makes its buckets `<[Vec<PatternID>; BUCKETS]>::try_from(vec)`,
which was refused: "calling `TryFrom::try_from`". A slice into an array
of its length was recognized (ADR 0189); a `Vec`, moved into one, wasn't.

## Decision

**`<[T; N]>::try_from(v)` and `v.try_into()` of a `Vec` are ADR 0189's
conversion, `SliceToArray`, `owned`: the `Vec` itself, moved, where it's
`N` long, `Ok(v)`; else `Err(v)`, the `Vec` given back.** It's no copy,
as nothing else has the `Vec`.

## Why

`vec_into_array` converts `Vec`s of `Vec`s, of the crate's structs and of
bytes, by `try_from` and `try_into`, one of the wrong length whose `Err`
gives it back, and changes an item after, against native Rust.

## Consequences

- Mutations refuse it, copy the moved `Vec`, and give back nothing in its
  `Err`.
