# 0936. An iterator extension's bound on a parameter's associated type is given its own dictionary

Status: Accepted, under the user's goal of 2026-10-26 ("keep going until
all done, the choice is your recommendation"). The plan "rust-js: one
representation per iterator", its second phase. Extends
[0487](0487-an-iterator-given-as-it-is-has-its-extensions-dictionaries.md)
and [0552](0552-an-into-iterator-held-as-it-is-is-given-as-it-is.md).

Case: C ([0262](0262-when-rust-and-js-disagree.md)): results change.
With std's structs alone, `groups.iter().flat_map(|g| g.iter().map(f)).rev()`
threw `it.back is not a function`.

## Context

std's `FlatMap` steps from its back where `U: IntoIterator<IntoIter:
DoubleEndedIterator>`: a bound on `U::IntoIter`, an associated type, which
a body was given no dictionary of, as only a bound on a type parameter
itself was (ADR 0487). Its evidence was found another way, as what the
`IntoIterator` dictionary carries, `IntoIterIterator()`, an `Iterator`'s,
whose `next_back` is none, or `{ next_back: $nextBackSome }`, which
assumed the iterator knew its own back: std's `Map` a closure made
doesn't.

## Decision

- **An iterator extension's bound on a type parameter's associated type,
  `U::IntoIter: DoubleEndedIterator`, is given its own dictionary**, as a
  bound on the parameter is, made by the caller who knows the type.

## Consequences

- ADR 0908's `differs` is 9, from 10: `flatten_of_iterators_from_back.rs`
  runs as native Rust does with std's structs alone.
- A generic function of the crate's bound so takes it: `fn rev_all<I>(i:
  I) where I: IntoIterator, I::IntoIter: DoubleEndedIterator` is
  `revAll(iterable, IIntoIterator, IIntoIterDoubleEndedIterator)`, given
  `{ next_back: $nextBack }` (`associated_double_ended.js`, and the
  compiled `FlatMap`'s back in `std_flat_map.js` and
  `flatten_from_its_back.js`): 19 lines in, 8 out.
- Mutations: `projection-extension-ungiven`.
