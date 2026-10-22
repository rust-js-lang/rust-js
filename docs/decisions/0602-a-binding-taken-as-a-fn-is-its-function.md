# 0602. A binding taken as a `fn` is its JS function, read where it's taken

Status: Accepted. Amends [0039](0039-generic-bindings.md)'s binding as a
value.

Case ([0262](0262-when-rust-and-js-disagree.md)): C, results would change:
Rust's, a `fn` fixed once it's taken.

## Context

React's scheduler captures what it calls, in case a polyfill overrides it:

```js
const localSetImmediate =
  typeof setImmediate !== 'undefined' ? setImmediate : null;
```

and its test "does not use setImmediate override" sets `setImmediate` after
the scheduler loads, which it mustn't call. ADR 0039 made a binding used
as a value an arrow of its own parameters, `(callback) =>
setImmediate(callback)`, so `.map(parseInt)` isn't given each index. That
arrow reads the global at each call: the polyfill was called.

## Decision

The user's choice: **a binding taken as a `fn`, a function pointer, is the
JS function itself, read where it's taken**, `const localSetImmediate =
setImmediate`, as a Rust `fn` is the one function from where it's taken.
Where its arrow does more than pass its parameters on, a `Result`'s
`try`, it stays that arrow.

**A binding handed to a JS callback itself is still its arrow**,
`.map(encode)` `.map((text) => encodeURIComponent(text))`: that's its
function item, not a pointer. And what hands a `fn` pointer to JS gives it
no more than Rust does, as it did: `.map((item) => parse(item))`.

## Consequences

- A binding's arrow didn't make its arguments as its call does, a
  `#[rust_js::nullable]` parameter's `null`: found here, fixed by
  [0604](0604-a-bindings-value-gives-what-its-call-gives.md).

## What holds it

- `test/lowering.test.ts`, "a binding taken as a fn is its JS function,
  read where it's taken": a global set after the module loads isn't
  called, and a pointer handed to `map` gets no index.
- The guide's [binding as a value](../guide/interop/bindings.md#a-binding-as-a-value).
- Mutations: the arrow kept, the cast not asked.
