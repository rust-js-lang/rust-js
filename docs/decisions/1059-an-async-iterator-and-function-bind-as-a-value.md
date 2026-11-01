# 1059. An async iterator, and `Function.prototype.bind` as a value

Status: Accepted (rd507, rd510, P2, 2026-11-02). Extends
[0860](0860-the-iterator-protocol-and-a-loop-of-what-its-test-makes.md)
(JS's iterator protocol) and
[0622](0622-a-global-binding-a-module-names-is-its-const.md), amended (a
private `use` that renames a global is the module's `const`).

Case ([0262](0262-when-rust-and-js-disagree.md)): N, no observable
difference: each binding is JS's own value or method, called as JS calls
it.

## Context

React's Flight reply client serializes an async iterable by stepping
what its `[Symbol.asyncIterator]()` gives, `iterator.next().then(progress,
reject)`, each step a promise. The js crate bound the sync protocol only,
`js::Iterator<T>`, so the port declared a stand-in `AsyncIterator` with a
`next` link of its own (rd507), which a port mustn't install.

It also keeps `const FunctionBind = Function.prototype.bind;` and calls
`FunctionBind.apply(this, arguments)` where a server reference's `bind`
is given another function. The js crate had `Array.prototype.slice` as a
value, `js::array::SLICE` (rd332), and no `bind`; the port read it off
another function, `js::get(js::unknown(SLICE), "bind")` (rd510).

## Decision

- **`js::AsyncIterator<T>`**, TypeScript's `AsyncIterator<T>`, whose
  `next()` gives a `js::Promise<js::IteratorResult<T>>`, the same step
  `js::Iterator<T>`'s gives, as a promise. Only the protocol, as the sync
  one is: an async generator is one, and so is any `{ next() }` that
  gives promises.
- **`js::function::BIND`**, `Function.prototype.bind` as a value,
  TypeScript's `Function`, as `js::array::SLICE` is `slice`'s:
  `#[rust_js::name = "FunctionBind"] use js::function::BIND as
  FUNCTION_BIND;` is upstream's `const`, and
  `function::apply(FUNCTION_BIND, this, Some(js::arguments()))` its
  `FunctionBind.apply(this, arguments)`.

## Why

- Each is TypeScript's own type or value, named as the js crate names its
  neighbours: `AsyncIterator` beside `Iterator`, `BIND` beside `SLICE`.

## Alternatives

- **`next()` as a Rust `Future`**: `await` of a step reads well in Rust,
  but upstream chains `.then(progress, reject)`, which a promise already
  gives (`then_with`), and `.await` of a `js::Promise` is one too.

## Consequences

- Held by `test/js-builtins.test.ts`, "an async iterator's next gives a
  promise of each step", of an async generator in Node, and
  `test/lowering-binding-paths.test.ts`, "Function.prototype.bind is a
  value a private use names". Bindings, not compiler code: no mutation
  guards them; a wrong link name fails each test.
