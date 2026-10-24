# 0698. A `catch` handler gives a value or a promise, as TypeScript's

Status: Accepted. Amends [0283](0283-js-crate-covers-typescripts-es-library.md),
whose `Promise::catch` took a handler that gives a promise, as
ReScript's.

Case ([0262](0262-when-rust-and-js-disagree.md)): N: JS's `catch`,
which fulfils with what its handler gives, waiting for a promise.

## Context

React's `shared/ReactComponentStackFrame.js` silences an async
component's rejection:

```js
if (maybePromise && typeof maybePromise.catch === 'function') {
  maybePromise.catch(() => {});
}
```

`Promise::catch` took `impl FnOnce(&JsError) -> Promise<T>`, so the
port wrote `promise.catch(|_| js::promise::resolve(None))`, which printed
`promise.catch(() => Promise.resolve(undefined))`. TypeScript's
`onrejected` gives `TResult | PromiseLike<TResult>`.

## Decision

**`catch`'s handler gives what fulfils the promise, `R: Fulfils<T>`: a
`T`, a `Promise<T>`, which JS waits for, or, of an `Option`'s, nothing,
`()`, which is `undefined`, its `None`.** The port writes:

```rust
if let Some(maybe_promise) = maybe_promise
    && let Kind::Function(_) = js::classify(js::get(maybe_promise, "catch"))
{
    unsafe { js::cast::<Promise<Option<&'static Unknown>>>(Some(maybe_promise)) }.catch(|_| {});
}
```
```js
if (maybePromise != null && typeof maybePromise.catch === "function") {
  maybePromise.catch(() => {});
}
```

- **A thenable, any value with a `catch`, is upstream's test**:
  `Kind::Function(_)` of `js::get(value, "catch")`, `typeof value.catch
  === "function"`; then the value is a promise to the program, a cast,
  `js::cast::<Promise<..>>`, which is the value itself, written in the
  call: no name of its own.
- **A `catch` whose promise nothing reads is a statement**,
  `promise.catch(..);`: `Promise` isn't `#[must_use]`, so rustc warns
  nothing, and clippy's `let_underscore_future` and `drop_non_drop` are
  of `let _ =` and `drop(..)`, which aren't written.
- **The promise it gives is a `Promise<T>`**: TypeScript's `T |
  TResult`, where `TResult` is `T`, as Rust has no union of them.

## Why

- **TypeScript's handler**, which gives a value or a promise; the port
  writes upstream's `() => {}`.
- **One `catch`**: a handler of each is the one method, as JS's is.

## Alternatives

- **`catch_resolve`, for a handler of a value, as `then_resolve` is
  `then`'s**: two methods of one JS method.
- **`()` fulfils any promise**: of a `Promise<u32>`, `undefined` isn't a
  `u32`, so only an `Option`'s is fulfilled by nothing.

## Consequences

- `then` still takes a handler of a promise, and `then_resolve` one of a
  value, as ReScript's: `then`'s `U` is inferred from what its handler
  gives, which a `Fulfils` couldn't tell of `()`.

## What holds it

- `test/js-builtins.test.ts`, "a promise's catch handler gives a value
  or a promise": `maybePromise.catch(() => {});` of a cast, with no
  warning; a handler of a value, of a promise and of nothing, each
  awaited; and a rejection silenced, which Node doesn't report.
- `test/js-object.test.ts`, "React's component stack frame is found as
  upstream finds it", of an async component.
