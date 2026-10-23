# 0656. An event's `emit` arguments are what TypeScript's `emit` takes

Status: Accepted. Amends [0361](0361-node-events.md), whose
`uncaughtException`'s arguments were its listener's.

Case ([0262](0262-when-rust-and-js-disagree.md)): N: the call is JS's.

## Context

React's `reportGlobalError` polyfills Node's by emitting what an uncaught
error would:

```js
process.emit('uncaughtException', error);
```

`Emits<UncaughtException>`'s `Args`, what `process::emit` is given, was
its listener's, `(&JsError, UncaughtExceptionOrigin)`, so the port had to
give an origin. `@types/node` types the two apart: the listener is given
`(error: Error, origin: UncaughtExceptionOrigin)`, and `emit(event:
"uncaughtException", error: Error)` takes the error alone. Node gives the
origin to the listeners it calls itself.

## Decision

**An event's `Args` are what `@types/node`'s `emit` overload of it
takes**, as its `Listener` is what its `on` overload's listener is given:

```rust
process::emit(UncaughtException, (error,))
```

```js
process.emit("uncaughtException", error);
```

Each of `process`'s, checked against `emit`'s overloads:

| Event | `emit` takes | `Args` |
|---|---|---|
| `beforeExit`, `exit` | `code: number` | `(f64,)` |
| `disconnect` | nothing | `()` |
| `rejectionHandled` | `promise` | `(&AnyPromise,)` |
| `uncaughtException`, `uncaughtExceptionMonitor` | `error: Error` | `(&JsError,)`, was `(&JsError, UncaughtExceptionOrigin)` |
| `unhandledRejection` | `reason, promise` | `(Option<&Unknown>, &AnyPromise)` |
| `warning` | `warning: Error` | `(&JsError,)` |
| `workerMessage` | `value, source` | `(Option<&Unknown>, f64)` |
| a signal | `signal?: Signals` | `(Signals,)` |

## Why

- **TypeScript's `emit`**: a binding types a call as TypeScript does, and
  its `emit` and its listener are typed apart.
- **No optional argument was needed**: `@types/node` 24 gives
  `uncaughtException`'s `emit` no `origin`, optional or not.

## Alternatives

- **An optional trailing argument, `(&JsError, Option<..>)`**: an
  argument `emit` doesn't take; and a rest tuple's `None` is an argument,
  `undefined`, not one left out (ADR 0640).

## Consequences

- **A signal's `emit` needs its `signal`**, which TypeScript lets a
  caller leave out: a rest tuple has no optional item yet, and its
  trailing `None` is an argument (ADR 0640), as `console.log(x, None)`
  must keep it.

## What holds it

- `test/node-bindings.test.ts`, "node's events are typed by their
  emitters": `process::emit(UncaughtException, (error,))` is
  `process.emit("uncaughtException", error)`, a listener given the error,
  run by Node.
