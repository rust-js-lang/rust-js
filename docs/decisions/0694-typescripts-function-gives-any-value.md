# 0694. TypeScript's `Function` gives any value

Status: Accepted. Amends [0653](0653-a-js-function-value-is-a-dyn-fn-of-its-signature.md),
whose TypeScript `Function` was `&dyn Fn()`, which gives nothing.

Case ([0262](0262-when-rust-and-js-disagree.md)): N: each is the JS
function it was, called as JS calls it; what it gives is what JS gives.

## Context

React's `shared/ReactComponentStackFrame.js` calls a component for what
it gives:

```js
export function describeNativeComponentFrame(fn: Function, construct: boolean): string {
  ..
  const maybePromise = fn();
  if (maybePromise && typeof maybePromise.catch === 'function') {
```

ADR 0653 made TypeScript's `Function`, a function of whatever it takes,
`&dyn Fn()`, as `js::Kind::Function` held it. A `dyn Fn()` gives `()`, so
the port cast it to call it, `unsafe { js::cast::<&dyn Fn() ->
Option<&Unknown>>(..) }`, which printed `const call = fn; const
maybePromise = call();`: a cast, and a name upstream hasn't. TypeScript
types a call of a `Function` as `any`.

## Decision

**TypeScript's `Function` is `&dyn Fn() -> Option<&'static Unknown>`,
one form everywhere: called of nothing, what it gives is any value, an
`any` result as ADR 0653 has it.** The port writes:

```rust
pub fn describe_native_component_frame(r#fn: &dyn Fn() -> Option<&'static Unknown>, construct: bool) -> String {
    ..
    let maybe_promise = r#fn();
```
```js
const maybePromise = fn();
```

| Binding | Was | Is |
|---|---|---|
| `js::Kind::Function` | `&'a dyn Fn()` | `&'a dyn Fn() -> Option<&'static Unknown>` |
| `reflect::apply(target, ..)` | `&dyn Fn()` | the same |
| `reflect::construct(target, .., new_target)` | `&dyn Fn()`, `Option<&dyn Fn()>` | the same |
| the `WeakKey` of a function | `dyn Fn()` | the same |
| node's `CallSite.getFunction()` | `Option<&'static dyn Fn()>` | the same |
| node's `Error.captureStackTrace(.., constructorOpt?)` | `Option<&dyn Fn()>` | the same |
| webapi's 2 `QueuingStrategy.size`, WebIDL's `Function` | `&'static dyn Fn()` | the same |

Eight bindings and a key moved, in three crates; the webapi generator
writes the new one. The JS of each is what it was. The tests that named
the old type, a weak map of functions and `captureStackTrace`'s, name
the new one; the rest matched `Kind::Function(_)`, unchanged.

- **A Rust function given as one gives a value too**: `fn fake() ->
  Option<&'static Unknown> { panic!() }`, whose `panic!()` gives any type.
  One that gives nothing, `fn f() {}`, isn't a `Function`, as Rust's
  `fn()` isn't a `fn() -> T`.
- **Its arguments are still none**: a call with some is `reflect::apply`,
  `Reflect.apply(f, this, [a])`, as it was.

## Why

- **TypeScript's own type**: a `Function` called is `any`, and a port
  reads what it gives with no cast.
- **One type**: the type `Kind::Function` holds is the one each binding
  takes, so a function read is one given, kept or compared.

## Alternatives

- **A generic parameter of any function, `&F` where `F` is any `Fn`**,
  for what a binding takes, beside `Kind`'s type for what it gives: two
  types of one TypeScript type, and an optional one, `Option<&F>`, which
  `None` can't infer.
- **`&dyn Fn() -> R`, generic in `R`**: no one type for `Kind` to hold.
- **Keep `dyn Fn()`**: a cast to read what every call gives.

## Consequences

- A `Function`'s declaration is `() => unknown | null | undefined`, what
  its Rust type is, where TypeScript writes `Function`.
- A program that held a `&dyn Fn()` from `Kind::Function`, or gave one to
  `Reflect`, names the new type; `fn` items it gives return a value.

## What holds it

- `test/js-object.test.ts`, "TypeScript's Function gives any value": a
  function `Kind::Function` holds called, `value()`, what it gives read;
  `Reflect.apply` of one; a Rust `fn` given as one; run.
- `test/js-builtins.test.ts`, "a weak map is keyed by what js::Kind
  holds", and `test/node-bindings.test.ts`, "V8's stack trace hook is
  saved, set and restored", of the new type.
- `test/webapi-coverage.test.ts`, "the webapi crate is what its generator
  writes".
