# 0921. A `&mut` to a generic iterator an `Option` holds is the iterator, whichever method gives it

Status: Accepted. Amends [0486](0486-an-option-of-a-boxed-payload-is-changed-in-place.md);
follows [0071](0071-stepping-iterators.md) and [0099](0099-mut-references.md).

Case: C ([0262](0262-when-rust-and-js-disagree.md)): results change.
`slot.get_or_insert_with(f).next()` of an `I: Iterator` gave `None` where
Rust gives the iterator's first item, and `slot.insert(x.into_iter()).next()`
threw `it.next is not a function`.

## Context

A `&mut` to a value JS can't change in place is a cell, a handle on its
place; a `&mut` to a generic iterator is the JS iterator it steps (ADR
0071), which `is_cell_pointee` answers. An `Option` of a generic type
boxes what it holds where it could look like `None` (ADR 0051), and a
pattern, `if let Some(it) = slot`, or `as_mut()`, binds a `&mut` to it by
that rule: `$someValue(slot.value)`, the iterator, of a box.

`insert`, `get_or_insert` and `get_or_insert_with` asked only whether the
payload was boxed: a boxed one's `&mut` was always `$someMut(handle)`, a
handle that reads and writes through the box. Given to an iterator's
`next`, `$rangeNext` or the crate's own, it stepped the handle: no item,
or no `next` to call.

## Decision

- **What these methods give is a `&mut` by the one rule**,
  `is_cell_pointee` (`option_place_through`): of a cell's payload, a
  handle, `$someMut(..)` where it's boxed; of what's no cell, the value,
  `$someValue(place)` where it's boxed, as a pattern binds it.

## Consequences

- `option_slot_iterators.rs` runs as native Rust does: a generic
  iterator stepped through each method, and a generic `T`'s and a
  number's `&mut` still handles that write the `Option`'s place.
- std's `FlattenCompat::iter_try_fold`, whose `frontiter.insert(..)` gives
  its inner iterator, is no longer refused for it.
- Mutations: `option-slot-iterator-a-handle`, `option-slot-number-not-a-handle`.
