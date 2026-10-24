# 0586. `#[rust_js::nullable]` is of a field that's an `Option`

Status: Accepted. Extends [0196](0196-typescript-declarations.md)'s
nullable fields and [0275](0275-nullable-writes-null.md). Amended by
[0772](0772-a-nullable-cell-field-writes-null.md): or a `Cell` of one.

Case ([0262](0262-when-rust-and-js-disagree.md)): none; a program rustc
accepts that rust-js can't give its meaning is refused.

## Context

`#[rust_js::nullable]` says a field's `None` is `null`. React's
scheduler's port put it on a `RefCell<Option<Callback>>`: the attribute
was taken of the cell, `({ value: callback } ?? null).value`, and of no
`None`. A function's `nullable` naming what isn't an `Option` was already
an error; a field's wasn't.

## Decision

**`#[rust_js::nullable]` of a field that isn't an `Option` is a compile
error**, as a function's is:

```
rust-js: `#[rust_js::nullable]` is of a field that's an `Option`, whose `None` is `null`; this one isn't
```

## Why

- **An attribute that does nothing it says is a silent difference**,
  which the reader can't see in the JS.

## What holds it

- `test/diagnostics.test.ts`, "a nullable field that isn't an Option".
- Mutation: the check gone.
