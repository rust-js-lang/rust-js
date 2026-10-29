# 0782. A promise's executor may borrow, and `then` takes two handlers

Status: Accepted. Amends [0283](0283-js-crate-covers-typescripts-es-library.md)'s
`Promise`, as [0698](0698-a-catch-handler-gives-a-value-or-a-promise.md)
amended its `catch`. Amended below: a promise of a `Copy` value is
`Copy`; what rejects one takes any value. Amended again below: what was
caught, a `&JsError`, is a value of any shape, `error.as_unknown()`.

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

## Amendment: a promise is a handle, and its reason any value

React's `lazy()`, ported further, keeps the promise in `ioInfo.value` and
hides its rejection later, then rejects it with what the loader rejected
with, a `mixed`:

```js
ioInfo.value.then(noop, noop);
rejectDebugValue(error);
```

Two gaps:

- **`then` consumed the promise.** `Promise` was neither `Copy` nor
  `Clone`, and `then`, `then_with`, `catch`, `finally` and `then_resolve`
  take `self`, so a promise a struct keeps, `Cell<Option<Promise<..>>>`,
  couldn't be read to call one (`Cell::get` needs `Copy`), and one behind
  a `&'static`, `info.value.get().unwrap().then_with(..)`, was E0507. A JS
  promise is a handle: `p.then(f)` leaves `p` as it was.
- **`reject` took only a `&JsError`**: `new`'s and `with_resolvers`'s
  `&'static dyn Fn(&JsError)`, and `Promise.reject(reason: &JsError)`,
  where TypeScript's is `reason?: any`, so `rejectDebugValue(error)` of a
  `mixed` error couldn't be written.

Case ([0262](0262-when-rust-and-js-disagree.md)): N for each. A copy of a
promise is the one JS promise, read by each handler; each handler of a
`Copy` value gets a copy of it, as Rust's would, so sharing it changes
nothing a program sees. Of a value that isn't `Copy`, sharing would
(below), so it isn't shared.

### Decision

- **A promise of a `Copy` value is `Copy`** (`impl<T: Copy> Copy for
  Promise<T>`, and `Clone` of it), and its methods still take `self`, as
  `.await` does. `io_info.value.get().unwrap().then_with(..)` reads
  `$unwrap(ioInfo.value).then(..)`; the port keeps `Cell<Option<Promise<
  Option<&'static Unknown>>>>`, no `Box::leak`. A copy is the JS promise
  itself: `copies.rs` copies no JS object, and one that's `Copy` is
  `shareable`, as react's `ElementType` is (ADR 0234).
- **One of any other value is consumed, as it was**: `p.then_resolve(|mut
  v| v.push(1)); p.then_resolve(|v| v.len())` of a `Promise<Vec<u32>>` is
  rustc's E0382, since in JS both handlers would get the one array, and
  the first's push would change what the second sees.
- **What rejects a promise takes any value.** `new`'s and
  `with_resolvers`'s `reject` is `&'static dyn Fn(Option<&Unknown>)`, a
  JS function value's `any` argument as ADR 0653 has it, so
  `reject_debug_value.unwrap()(error)` of an `Option<&'static Unknown>` is
  `rejectDebugValue(error)`; `promise::reject(reason: Option<&dyn
  IntoUnknown>)`, a binding's optional `any` parameter as the guide's
  bindings table has it (ADR 0443): `promise::reject(Some(error))` of a
  `&JsError`, `promise::reject(Some(&value))` of any other, and
  `promise::reject(None)` is `Promise.reject()`. A handler still reads a
  reason as a `&'static JsError`, whatever was thrown, as `settle` does.

### Alternatives

- **Methods on `&self`, as a JS object's are** (webapi's): unsound of a
  promise of a value that isn't `Copy`, whose handlers would each own the
  one array; and `&self` methods only where `T: Copy`, with `self` ones
  otherwise, are two of each method.
- **`reject` of `Option<&dyn IntoUnknown>`, as `Promise.reject`'s**: a
  `&JsError` would be given as `Some(error)` there too, but a JS
  function's `any` argument is `Option<&Unknown>` everywhere else, a
  descriptor's `set` and node's handlers (ADR 0653): two types for one
  rule.

### Consequences

- **A `&JsError` isn't given to `new`'s `reject` as it is**: it may be
  anything thrown, `undefined` too, so it isn't `Defined`, and
  `js::unknown` takes no `JsError`. No program rejects with one yet; one
  that does is the next change, a view of a `&JsError` as the
  `Option<&Unknown>` it is, which every function of an `any` argument
  needs alike, not a second `reject`.
- **A handler of fewer parameters isn't bound.** Upstream gives `then` a
  function of none, `noop`; Rust's `then_with` takes an `FnOnce(T)`, so
  `then_with(noop, noop)` is E0593, and the port writes `then_with(|_|
  noop(), |_| noop())`, `() => { noop(); }` each. Neither general rule
  that would print `noop` holds: printing a closure that only calls a
  function as the function changes which function JS is given, which JS
  sees, `addEventListener` keeping one listener of `noop` given twice,
  where Rust's two closures are two; and a handler trait implemented for
  functions of each arity, as `AnyShape` is, leaves a closure's
  parameters untyped, `|n| n.to_string()` an error, since rustc infers
  them only of an `Fn` bound. It stays open.

### What holds it

- `test/js-builtins.test.ts`, "a promise's executor may set the variables
  around it, and then takes two handlers": a promise kept in a nullable
  `Cell`, read twice, given two handlers and rejected with a `mixed`
  value, `rejectDebugValue(error)`; one of a `u32` given handlers twice;
  `Promise.reject(error)` of a `&JsError` and `Promise.reject()` of none,
  run in Node; and a `Promise<Vec<u32>>` given two handlers, rustc's
  E0382.

## Amendment: what was caught is a value of any shape, `as_unknown()`

React's `cache` stores the first error a function threw, and throws it
again at each later call:

```js
} catch (error) {
  erroredNode.s = ERRORED;
  erroredNode.v = error;
  throw error;
}
```

The port's node keeps `v: Cell<Option<&'static Unknown>>`, any value,
and what `catch_unwind` caught downcasts to a `&'static JsError`, what
was thrown, which isn't `Defined`, as it may be `undefined` (above's
Consequences): `.map(|error| js::unknown(*error))` was E0277. Those
Consequences named the fix: one view of a `&JsError` as the
`Option<&Unknown>` it is, which every parameter of any value takes.

Case ([0262](0262-when-rust-and-js-disagree.md)): N: the view is the
value itself.

### Decision

**`JsError::as_unknown(&self) -> Option<&'static Unknown>` is what was
thrown as a value of any shape, `None` of `undefined` and `null`: the
value itself.** It's the one view; a parameter of any value, an
`Option<&Unknown>`, takes it, and an `Option<&dyn IntoUnknown>` takes the
`&JsError` as it is, `Some(error)`.

```rust
errored_node.v.set(error.downcast_ref::<&'static JsError>().and_then(|error| error.as_unknown()));
panic::resume_unwind(error)
```

```js
erroredNode.v = error;
throw error;
```

- **Of a falsy value too**: `0`, `""`, `undefined` and `null` are stored
  as they were thrown, as the closure of `and_then` is given what was
  thrown (ADR 0710), and `js::throw` of the field throws it again.

### Alternatives

- **`JsError: AnyShape`, `js::unknown(error)`**: an `&Unknown` is never
  `undefined` nor `null`, which what was thrown may be.
- **`impl From<&JsError> for Option<&Unknown>`, `.into()`**: a
  conversion no reader sees the meaning of at the call, `(*error).into()`.

### What holds it

- `test/js-builtins.test.ts`, "what was caught is a value of any shape":
  an `Error`, an object, a string, `0`, `""`, `undefined` and `null`, each
  stored in a field and thrown again, the same value.
  `test/rest-parameters.test.ts`, React's `erroredNode.v = error`, run
  against upstream's.

## Amendment: `then` of a value or a promise, TypeScript's (the user's decision, 2026-10-29)

`p.then(f)` took only a handler that gives a promise, `then_resolve(f)`
one that gives a value: two ways of TypeScript's one `then(onfulfilled)`,
whose handler gives `TResult | PromiseLike<TResult>`, narrower each than
its type. React DOM's readAsDataURL returns its text from
`blob.arrayBuffer().then(arrayBuffer => { .. return 'data:' + .. })`.
**`then` takes a handler that gives a value or a promise of one,
[`Fulfils`], as `catch` and `then_with` do; `then_resolve` is gone**
(`builtins/src/promise.rs`): one way, no narrower than TypeScript's.
Where rustc can't tell which a handler gives, a chain's next handler's
parameter or the promise it makes says, `.then(|n: u32| n * 10)`, `let p:
Promise<String> = ..`. Every caller in rust-js and react-in-rust inferred
it already but `then_resolve`'s five, now `then`'s. Held by
`test/js-builtins.test.ts` ("a promise's then handler gives a value or a
promise"), which a `then` narrowed again doesn't compile.

## Amendment: a promise of any value is `into_unknown()` (rd211)

React DOM's startViewTransition collects `Array<Promise<any>>`, fonts'
`ready` (a `Promise<FontFaceSet>`) beside images' `Promise<void>`s, and
races `Promise.all(blockingPromises)` against a timeout's promise;
`Promise.allSettled([navigation.finished, blockingReady])` too. A Rust
`Vec` holds one type, and nothing viewed a `Promise<T>` as TypeScript's
`Promise<any>`, `Promise<Option<&'static Unknown>>`: the port cast,
`js::cast(Some(js::unknown(&p)))`.

**`promise.into_unknown()` is the promise itself, what it fulfils any
value**, `Promise<Option<&'static Unknown>>`: a binding of the form
`this`, so nothing's written of it. A `From` impl can't say it, as it
would overlap `From<T> for T` of a promise of any value already.
`then(|v| js::unknown_of(v))` would be a new promise, a tick later.

Held by `test/js-any-values.test.ts`'s "a promise of any value is the
promise itself" (a list of two promises' values, raced against a timeout,
run both ways). No mutation: a binding's.

