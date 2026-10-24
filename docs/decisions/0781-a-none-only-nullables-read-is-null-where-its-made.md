# 0781. A `None` only nullables read is `null` where it's made

Status: Accepted. Extends [0275](0275-nullable-writes-null.md), whose
variable each write of which is an `Option` made, and each read a
nullable field's, was already `null` where it's made, to a variable MIR
copies before a field or a binding's parameter takes it. Amended by
[0841](0841-a-nullable-place-stores-what-is-never-undefined-as-it-is.md):
a call's `Option` only they read is written so where the call's made.

Case ([0262](0262-when-rust-and-js-disagree.md)): A: `null` and
`undefined` are each `None` to the program, which reads either as one
(ADR 0030); only what JS sees of the variable changes.

## Context

React's `lazy()` names the owner it doesn't track yet:

```js
const owner = null;
const ioInfo = {
  name: 'lazy',
  ..
  owner: owner,
```

Its port, `let owner = None;` and `ReactIOInfo { .., owner, .. }` of a
`#[rust_js::nullable]` field, printed `const owner = undefined;` and
`owner: owner ?? null`. The variable was one ADR 0275's rule makes `null`
(`OptionWrites::null_nones`): its one write a `None`, its one read the
field's. But MIR reads a variable a struct literal names through a copy,
`_2 = copy _1; Info { owner: move _2 }`, so its read was the copy's, not
the field's.

## Decision

**A copy of a variable, made once and only read by nullable fields and
binding parameters, gives the variable to them**: `let owner = None;`
whose every read is such a copy, or such a field or parameter, is `null`
where it's made, `const owner = null;`, and each field or argument takes
it as it is (`patterns.rs`'s `mir_null_nones`; `rvalues.rs`'s `nulled`
already follows the copy). **A binding's parameter it names
`#[rust_js::nullable(..)]` reads it as a field does**, `Object.is(owner,
1)`, which `nulled` already took as it is.

A variable read otherwise too, `owner.is_none()`, or copied to one read
otherwise, `let other = owner;` returned, stays `undefined`, and each
nullable field takes it as `owner ?? null`.

## Why

- **React's JS**: `const owner = null;`, no `?? null`.
- **One rule**: ADR 0275's, which a copy MIR makes no longer hides.

## Alternatives

- **Every `None` variable of a nullable type `null`**: one read by
  `is_none()` or returned is `undefined` to its JS reader, as every
  `None` is.

## What holds it

- `test/lowering-nullable.test.ts`, "a None only nullable fields and
  parameters read is null where it's made": a leaked struct's field, two
  fields, a binding's parameter, one also tested, one copied and
  returned, each run.
- Mutations `null-none-copy-ungiven`, `null-none-param-ungiven`,
  `null-none-copy-read-elsewhere` (`mir/patterns.rs`).
