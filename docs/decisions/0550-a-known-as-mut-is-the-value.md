# 0550. A known type's `as_mut()` is the value its `&mut` is

Status: Accepted. Amends [0533](0533-an-associated-types-as-mut-is-its-dictionarys.md).

Case: C ([0262](0262-when-rust-and-js-disagree.md)): what an `as_mut()`
writes is written to what it views, as Rust's is.

## Context

An impl of itertools' `TupleCollect` names `Self::Buffer`, an array there,
and calls `buf.as_mut()`. A call of std's `AsMut` went to a dictionary,
which takes the cell generic code's `&mut self` is (ADR 0533); of a type
known here, `buf` is the array itself, so its slice was read as the
cell's `value`, `undefined`.

## Decision

**std's `as_mut()` of a sequence as a slice, or of text as a `str`, of a
type known here, is the value its `&mut` is, `Std::Same`**, as `borrow()`
is (ADR 0167). A type known only by its bounds still asks its dictionary.

## Why

`impl_associated_as_mut` fills a buffer through an impl's `as_mut()`,
called directly and through a generic caller's dictionary, against native
Rust.

## Consequences

- itertools' `tuples()` runs.
- A mutation sends the known type's call to the dictionary.
