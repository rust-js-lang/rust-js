# 0900. A dictionary's method is the program's, whatever its name

Status: Accepted, under the user's delegation of 2026-10-25 ("proceed as
your recommendation"); open to their review. The review's P2: a fact of
Rust's on the JS tree in place of a JS shape read back. Extends
[0890](0890-a-bindings-method-is-known-as-one.md).

Case ([0262](0262-when-rust-and-js-disagree.md)): results could change, so
Rust's order is kept. No program of the corpus changes; the guard is for
the one that would.

## Context

ADR 0890 stopped `calls_only`, behind `only_reads` and
`runs_program_code`, from taking a binding's `store.join()` for JS's array
`join` by its name. A dictionary's method is the same case. A trait of the
crate's with a method named `join`, `map` or `values`, called in generic
code, is `TStore.join(t)`, a member call of one of those names, so it was
taken as JS's own, which only reads. Tidy may then move a value past it
(`first_applied`), and a format may read its order wrong
(`formats_out_of_order`).

## Decision

- **A dictionary the function was given is marked `Known::Evidence`**
  where its route is made (`route_expr`, `traits/routes.rs`), the one place
  a body names one: `IIterator`, `TStore`, a supertrait's or an associated
  type's dictionary reached through it.
- **`calls_only` never counts a call of a dictionary's method as JS's
  own**, whatever its name, as it doesn't a binding's (`Known::Binding`).

## Consequences

- Held by a unit test, `a_dictionarys_method_runs_program_code`: no
  program's JS changes, std's included, so no mutation's corpus test could
  catch it.
- What's left of `calls_only`'s names is JS's own methods of what Rust's
  types say is text or an array, and of what a binding or a dictionary
  isn't.
