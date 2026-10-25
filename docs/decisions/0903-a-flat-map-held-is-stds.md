# 0903. A `FlatMap` held is std's

Status: Accepted, under the user's delegation of 2026-10-25 ("proceed as
your recommendation"); open to their review. The review's P1: the last
iterator adapter ADR 0871 listed. Extends
[0899](0899-a-flatten-held-is-stds.md).

Case: C ([0262](0262-when-rust-and-js-disagree.md)): results change. A
held `flat_map()` was the recognizers' JS iterator, whose `size_hint` is
`(0, None)` where std's counts, as a held flatten's was.

## Context

`FlatMap<I, U, F>` is `FlattenCompat<Map<I, F>, U::IntoIter>`, so std's
bodies reach what ADR 0899 built for `Flatten`: the `OneShot` and
`ConstSize` facts, and the `IntoIterator` held as it is. Two more things
were missing:

- `U` is a type parameter of `FlatMap`'s impl, `U: IntoIterator`. Its
  `IntoIter` is held as it is, but in the struct's field, not the
  signature, so `into_iter_held_as_it_is` didn't see it.
- `FlattenCompat`'s back asks `Map<I, F>: DoubleEndedIterator`, which
  `Map`'s impl answers by `I: DoubleEndedIterator`. The stepped analysis
  passed on only a parameter given as it is, so `FlatMap`'s `next_back`
  wasn't given `I`'s.

## Decision

- **A held `flat_map()` is std's `FlatMap`**, stepped by its compiled
  `next`, `size_hint`, `fold`, `next_back` and `rfold`
  (`HELD_COMPILED`'s, not sized in place). One made in place stays the
  recognizers'.
- **A compiled std struct's fields count for what it holds as it is**
  (`held_by`): `FlatMap`'s `FlattenCompat<Map<I, F>, U::IntoIter>`.
- **What a picked impl asks of a std adapter it's given, it asks of
  that adapter's source** (`stepped_params`, `impl_asks`).
  `Map<I, F>: DoubleEndedIterator` asks `I`'s `DoubleEndedIterator`, and
  so its `Iterator`.

## Consequences

- `std_flat_map.rs` runs as native Rust does, every line: `Vec`s,
  arrays and `Option`s from both ends, sized as std sizes them, a
  function holding a value with a destructor, and one held in a struct
  from generic code. `flatten_from_its_back.js`' held `flat_map` is std's
  struct.
- Still refused, as on main: a held flat map of `Option`s of a type with
  a destructor (`closed_on_drop`'s inner iterator isn't closable).
- The readability baseline is blessed: `$iter` 86.7 to 87.9, the new
  case's sources; `Iterator.from` 15.3 to 15.4.
- Mutations: `flat-map-not-held`, `adapter-field-not-held`,
  `held-adapter-bounds-unasked`.
