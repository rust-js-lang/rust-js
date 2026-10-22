# 0612. Two values of any shape are equal by JS's `===`

Status: Accepted. Extends [0225](0225-unknown-values.md)'s `js::Unknown`.

Case ([0262](0262-when-rust-and-js-disagree.md)): C, results would change:
JS's answer, as a value of any shape is JS's alone.

## Context

React's `shared` code compares `mixed` values, anything at all, by `===`:
its `Object.is` polyfill's `x === y`, where `null` isn't `undefined`. A
`js::Unknown` had no `==`, and an `Option<&Unknown>`, which a value that
may be `null` or `undefined` is, compared by `==`, both `None`.

## Decision

The user's choice: **`impl PartialEq for js::Unknown` is `a === b`**, the
same object or the same value, `NaN` not itself, so it's `PartialEq`
alone, as an `f64`'s; **and `==` of two `Option<&Unknown>` is `===` of the
JS values**, so `null == undefined` is `false` as JS has it, where std's
`Option` would call both `None` equal. A value of any shape has no
meaning but JS's (`declarations::any_shape`, `js_eq`).

## Consequences

- `None == None` of `Option<&Unknown>` is `false` of a `null` and an
  `undefined`, unlike std's `Option`; of the same `None`, `true`.

## What holds it

- `test/lowering.test.ts`, "js::Unknown's == is ===, of an Option too".
- Mutations: refused, `==` of the `Option`s.
