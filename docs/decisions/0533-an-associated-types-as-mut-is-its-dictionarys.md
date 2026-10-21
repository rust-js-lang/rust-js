# 0533. An associated type's `as_mut()` is its dictionary's

Status: Accepted. Extends [0162](0162-generic-as-ref.md)
and [0180](0180-generic-writers.md).

Case: D ([0262](0262-when-rust-and-js-disagree.md)) closed: what was
rust-js's own error compiles as Rust's does.

## Context

A call of std's trait that takes a `&mut self`, `buf.as_mut()`, goes to its
dictionary where its `Self` is a type parameter's, `B: AsMut<[T]>`. Of an
associated type, itertools' `TupleBuffer`'s `T::Buffer`, bound by `AsMut`
where the trait declares it, it was refused, "calling
`std::convert::AsMut::as_mut`", though its `default()`, which takes no
`&mut`, was its dictionary's.

## Decision

**A `&mut self` call of std's trait on an associated type is its
dictionary's, as a type parameter's is**: either is known only by its
bounds. Its dictionary is the one a type parameter's `AsMut` is given, its
value in a cell.

**Whether a type is known only by its bounds is one question,
`recognition::known_by_bounds`.** Nine places wrote it in place, `ty::Param(_)
| ty::Alias(..)`, and this one wrote half of it; each asks it now, and the
architecture test holds that none writes it again.

## Why

`associated_type_as_mut` fills, reads and empties a `T::Buffer` by
`as_mut()` and `as_ref()`, of `String`s and of numbers, against native
Rust.

## Consequences

- itertools' `TupleBuffer` compiles.
- Mutations take a type parameter's alone, here and in `known_by_bounds`.
