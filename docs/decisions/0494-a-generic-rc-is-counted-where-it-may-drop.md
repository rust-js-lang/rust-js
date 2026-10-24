# 0494. An `Rc` of a type parameter's value is counted where it may drop

Status: Accepted. Extends [0320](0320-counted-rcs.md). Amended by
[0844](0844-a-library-that-reads-no-count-counts-no-rc.md): of a parameter
that may have a destructor, not a `T: Copy`, where a body holds it, through
the fields of what it holds.

Case: C ([0262](0262-when-rust-and-js-disagree.md)): what's dropped, and
when, as std's, by the last clone of an `Rc`.

## Context

An `Rc` is counted, `{ value, strong, weak }`, where its counts are read or
its value's drop runs, which the last clone drops (ADR 0320); every other
is the value it points at. A generic `Rc`'s value, `Rc<RefCell<TeeBuffer<
I::Item, I>>>` as itertools' `tee` holds, was neither: the analysis saw no
type it points at, and its drop was refused, "a std type holding a value
with a destructor". And `VecDeque::pop_front` of a type parameter's item
was refused, its `Some` unboxed.

## Decision

**An `Rc` that holds a type parameter's value, not one behind a
reference, in a crate where a type may have a destructor, makes every `Rc`
counted, as a generic `Rc` that may be a counted one does (ADR 0320).**
Its last clone drops what it holds, by the drops its function's given.

- **Not a library's**: a counted `Rc` its consumers would share uncounted
  is refused there (ADR 0320), and is refused as before.
- **`pop_front` of a type parameter's items is `$shift`**, which boxes its
  `Some`, as `pop`'s `$pop` does.

## Why

- **It's std's**: `generic_rc_drops` tees two iterators over a shared
  buffer, of numbers, of items with a destructor dropped by the last
  clone, and of `Option`s, with native Rust; `rc_of_borrowed_param` holds
  an `Rc` of a reference, uncounted; `rc_count_through_generic` reads a
  count through generic code in a crate with no destructor.

## Consequences

- **A crate with a destructor and a generic `Rc` of a type parameter's
  value counts every `Rc`.**
- **A library's `tee`** is refused yet: the counting a library's `Rc` has
  must be its consumers' too, an answer to report across the crate
  boundary.
