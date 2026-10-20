# 0530. std's iterators of arrays, and a `rev`, clone where they are

Status: Accepted. Extends [0181](0181-iterator-clones.md),
[0364](0364-mir-lowering.md) and [0516](0516-rev-steps-what-it-holds-from-its-back.md);
accepts part of the [iterators draft](draft/cloned-and-buffering-iterators.md).

Case: C ([0262](0262-when-rust-and-js-disagree.md)): a clone is where the
iterator is, and its items its own where they change in place.

## Context

A std iterator over an array is a `$iter`, which knows where it is, and
`steps_an_array` is the one list of them. Cloning one asked another,
narrower answer, `array_source`, so a string's `chars()`, a `VecDeque`'s
`iter()` and others on the list weren't cloned; an `Option`'s and a
`Result`'s `iter()` were `$iter`s not on it; a `LinkedList`'s `iter()` and
a string's `char_indices()` were JS helpers, no `$iter` at all. A `rev()`
of an array's iterator is its array reversed, a `$iter`, but wasn't on the
list either, so its `next_back` was refused, as was any `rev()`'s clone.
itertools clones each of these, `peeking_next_by_clone!`. A `Box<[T]>`, an
array, couldn't be collected into or cloned.

## Decision

**`steps_an_array` is the one answer to "is this std iterator a `$iter` of
an array", which cloning asks too.**

- On it now: an `Option`'s and a `Result`'s `iter()`, a `LinkedList`'s,
  and a string's `char_indices()`, `iterates_items`; and a `rev()` of one
  on it, its array reversed. A `LinkedList`'s `iter()` is a `$iter` of its
  array, and `char_indices()` of the array `$charIndices(s)` makes, which
  isn't copied again where it's collected (`fresh`).
- A clone of one is a `$iter` of what it has left, each item a clone where
  the iterator owns them, a `rev()`'s by its source's ownership.
- **A `rev()` of another** is what its source makes it, by type: of a type
  parameter given as it is, the `$iterator` its `next_back` steps, made
  again of a clone of its value, `$iteratorWith`, as a `Fuse`'s is; of one
  that knows its back, `$rev(it)`, which keeps `it` for a clone to clone.
  It knows its back and its length where its source does, so a
  `memrchr_iter`'s `rev()` steps.
- **A `Box<[T]>` is an array**: `collect` makes one, and a clone of its
  slice is the array's, item by item where its items change in place.

## Why

`std_iterator_clones` clones each of them partway, steps them from either
end, a `rev()` of generic code's iterator and of a `Vec`'s whose items are
changed after the clone; `boxed_slice_values` collects into and clones a
`Box<[usize]>` and a `Box<[Vec<u8>]>`; memchr's probe clones a
`memrchr_iter` and reverses it; each against native Rust. itertools'
seven `peeking_next_by_clone!` refusals are gone.

## Consequences

- Five snapshots read better: `$charIndices(s)` and a `LinkedList`'s
  array, not copied first.
- Mutations take each off the list, refuse a `rev()`'s back or length,
  share a clone's items or source, take a given parameter's `rev()` for a
  `$rev`, and refuse a `Box<[T]>`'s collect and clone.
