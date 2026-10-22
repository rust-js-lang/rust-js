# 0604. A binding as a value gives its arguments as its call does

Status: Accepted. Amends [0039](0039-generic-bindings.md)'s binding as a
value.

Case ([0262](0262-when-rust-and-js-disagree.md)): C, results would change:
what JS is given is what its call gives.

## Context

A binding used as a value is an arrow of its own parameters (ADR 0039).
Its call made each argument as JS takes it: a `#[rust_js::nullable]`
parameter's `None` as `null` (ADR 0275), a `()` as `[]` (ADR 0020), a lazy
one's value (ADR 0593), a `&dyn Any`'s value (ADR 0331). Its arrow didn't:
`(to) => reset(to)` gave `undefined` where the call gives `null`, which JS
may tell apart, `to === null`. Found by ADR 0602's test.

## Decision

**A binding's arrow gives each argument as its call gives it**: both ask
one place, `binding_arg`, and make the call one way, `binding_values`,
`(to) => reset(to ?? null)`. A binding whose call is a statement, a
setter's, is no value, as before.

## What holds it

- `test/lowering.test.ts`, "a binding taken as a fn is its JS function,
  read where it's taken": a nullable binding's arrow gives `null`.
- Mutation: the arrow's arguments as given.
