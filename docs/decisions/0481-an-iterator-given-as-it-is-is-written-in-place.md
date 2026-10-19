# 0481. An iterator given as it is is written in place

Status: Accepted. Extends [0411](0411-an-iterator-given-as-it-is-has-its-dictionary.md)
and [0474](0474-an-iterator-another-bound-reads-is-given-as-it-is.md).

Case: A ([0262](0262-when-rust-and-js-disagree.md)): the same value, which
each name for it sees.

## Context

`*r = v` of a `&mut` to an object writes the object in place, `$assign(r,
v)`, so each name for it sees `v`; of what's neither an object nor a cell,
it was refused. A type parameter given as it is, an iterator another bound
reads (ADRs 0411, 0474), is its caller's own value, an object: the crate's
iterator, a `$iter`, a range. itertools' `Product` restarts its inner
iterator so, `*b = b_orig.clone()`, and it was refused.

## Decision

**`*r = v` of a `&mut` to a type parameter given as it is is `$assign(r,
v)`**, as of any object. One that isn't given as it is, a JS iterator, or a
function, is refused still: JS can't write either in place.

## Why

- **It's std's**: `iterator_written_through` restarts an inner iterator of
  the crate's and of a `Vec`'s, through generic code, with native Rust.

## Consequences

- **What compiles grows**: writing a whole iterator given as it is through
  a `&mut`.
