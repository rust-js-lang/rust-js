# 0548. A library's manifest says which type parameters it steps

Status: Accepted. Extends [0415](0415-a-crate-of-crates-io-in-an-app.md)
and [0511](0511-the-crates-iterator-extension-traits.md).

Case: C ([0262](0262-when-rust-and-js-disagree.md)): a library's generic
iterator steps what its caller gives it, as Rust's does.

## Context

A function's `I: Iterator` takes a dictionary only where the crate steps
`I` through it, an answer the crate's analysis gives (`stepped`). A
library's function was asked that question by its consumer, which can't
see the library's bodies: it gave no dictionary, and the library's `next`
had none to step by. And a default method of a library's extension trait,
`trait Ext: Iterator`, routed to its `Iterator` dictionary through the
supertrait, `SelfExt.Iterator()`, which a supertrait no dictionary holds,
a marker or one rust-js answers itself, doesn't have.

## Decision

**A library's manifest says, of each function, which type parameters it
steps by which trait, `stepped`, as the crate's own analysis does; a
consumer reads it, `Foreign::steps`, where it would ask the analysis.** The
manifest is version 4.

- **`held_supertraits` is the one list of the supertraits a dictionary
  holds**, which both a dictionary's entries and a route through one ask.
- A `copied()` of an array's iterator is one too (ADR 0547), on
  `steps_an_array`.

## Why

The two-crate test `extension_default` calls a library's generic
adapter, its extension trait's defaults and one stepping `Self` from the
consumer, against native Rust.

## Consequences

- A manifest of version 3 is refused, as one of another compiler is, and
  its library built again.
- Mutations leave the manifest's stepped facts unsaid and unread, and
  route through a supertrait no dictionary holds.
