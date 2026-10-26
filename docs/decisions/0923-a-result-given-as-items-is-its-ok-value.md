# 0923. A `Result` given as an `IntoIterator` is its `Ok` value, as its `into_iter()` is

Status: Accepted. Extends [0326](0326-option-and-result-methods.md).

Case: C ([0262](0262-when-rust-and-js-disagree.md)): results change.
`[1].into_iter().chain(Ok(6)).collect()` threw `TypeError: ... is not
iterable` where Rust gives `[1, 6]`, and so did `v.extend(result)`, a
`zip` of one and a generic `I: IntoIterator` given one.

## Context

What's given where an `IntoIterator` is taken is stepped as its items,
`iterable_items`: an `Option`'s are `x == null ? [] : [x]`, a B-tree's in
its keys' order. A `Result`'s weren't anything: it was given as the
`{ TAG, _0 }` object it is, which JS can't spread or step. Its
`into_iter()`, `for x in result` and `result.iter()` were right, each a
call of std's `Result::into_iter`, `Comb::ResultIter`.

## Decision

- **A `Result` given as an `IntoIterator` is its `into_iter()`**,
  `result.TAG === "Ok" ? [result._0] : []`, the one lowering of
  `Comb::ResultIter`, wherever it's given: `extend`, `chain`, `zip`,
  `flatten`'s sources, and a generic function's `IntoIterator`.

## Consequences

- `result_given_as_items.rs` runs as native Rust does, a `&mut` to one
  given to `chain` a handle on its `Ok` value, and a `Result` of a value
  with a destructor chained dropped where Rust drops it.
- ADR 0908's `differs` lists it, 21: with std's structs alone, a `Chain`
  of what has a destructor is refused, as `flat_map_drops.rs`'s `Flatten`
  is.
- Mutations: `result-items-unstepped`.
