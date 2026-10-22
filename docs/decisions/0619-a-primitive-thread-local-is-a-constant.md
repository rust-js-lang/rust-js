# 0619. A thread-local of a primitive is named as a constant

Status: Accepted. Amends [0579](0579-a-thread-local-is-named-as-a-variable.md).

Case ([0262](0262-when-rust-and-js-disagree.md)): A: only a name.

## Context

A thread-local is its module's variable, named as JS names one,
camelCase, `TASK_QUEUE` as `taskQueue` (ADR 0579). React's `ReactSymbols`
exports constants made when it loads, as JS names a constant:

```js
export const REACT_ELEMENT_TYPE = Symbol.for('react.transitional.element');
```

In Rust each is a thread-local, as a `const` can't call `Symbol.for`, and
was `reactElementType`; the scheduler's thread-locals of functions,
`localSetTimeout`, are camelCase upstream.

## Decision

The user's choice, by its value: **a thread-local of a primitive, a
number, text, a `bool` or a symbol, keeps its Rust name, as a `const`
does**, `REACT_ELEMENT_TYPE`; nothing writes one, which a `Cell` would be.
**One of a function or an object, or a `Cell`, is a camelCase variable**,
`localSetTimeout`, `taskQueue` (`bindings::primitive`). A `js::Symbol` is
TypeScript's `symbol`.

## What holds it

- `test/lowering.test.ts`, "a thread-local of a primitive is named as a
  constant", with one of a function and one of a `Cell`.
- Mutations: a primitive's camelCase, a symbol not taken for one.
