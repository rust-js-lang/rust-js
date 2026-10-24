# 0782. A promise's executor may borrow, and `then` takes two handlers

Status: Accepted. Amends [0283](0283-js-crate-covers-typescripts-es-library.md)'s
`Promise`, as [0698](0698-a-catch-handler-gives-a-value-or-a-promise.md)
amended its `catch`.

Case ([0262](0262-when-rust-and-js-disagree.md)): N: each is a binding's
Rust signature; the JS each call is, `new Promise(..)` and `p.then(f, g)`,
is JS's.

## Context

React's `lazy()` stashes a promise's resolvers to settle it later:

```js
let resolveDebugValue = null;
let rejectDebugValue = null;
ioInfo.value = new Promise((resolve, reject) => {
  resolveDebugValue = resolve;
  rejectDebugValue = reject;
});
..
ioInfo.value.then(noop, noop);
```

`js::promise::new`'s executor was `'static`, so it couldn't set the
`let`s around it; and `Promise` had `then(f)` of one handler only, where
TypeScript's `then` takes `onfulfilled` and `onrejected`.

## Decision

- **`promise::new`'s executor is an `FnOnce` of any lifetime**: JS runs
  it before `new Promise` returns, and keeps nothing of it, so what it
  borrows outlives its run; the `resolve` and `reject` it's given are
  `'static`, as JS keeps them.
- **`p.then_with(on_fulfilled, on_rejected)` is `p.then(f, g)`**: each
  handler gives a value or a promise of one, `Fulfils<U>`, as `catch`'s
  does, and the promise it makes is of `U`. The `_with` is the name a
  binding of more arguments than its one-argument form has
  (`json::stringify_with`); `then(f)`, of a handler of a promise, stays
  as it is.

A `then_with` whose promise nothing reads names its type, `let _:
Promise<()> = p.then_with(|_| {}, |_| {});`, as `()` fulfils a `()` and an
`Option`'s alike.

## Why

- **React's JS**, the resolvers set where upstream sets them.
- **TypeScript's `then`**, both handlers, which a port of
  `thenable.then(onFulfill, onReject)` of a promise needs.

## Alternatives

- **A `then` of an optional second handler**: Rust has no optional
  argument but an `Option`, `then(f, None)`, which every one-handler
  `then` would write.
- **`promise::with_resolvers()`** where upstream writes the executor: not
  upstream's JS.

## Consequences

- `console.createTask`, which React's `lazy()` also calls, is in no
  TypeScript library rust-js binds from (lib.dom.d.ts nor @types/node);
  only React's Flow environment types it, `createTask(label: string):
  ConsoleTask`. It's not bound: there's no TypeScript source to bind it
  from.

## What holds it

- `test/js-builtins.test.ts`, "a promise's executor may set the
  variables around it, and then takes two handlers": the resolvers set
  in the executor and settled later, `then_with` of each settling, and
  one whose promise nothing reads, run in Node.
