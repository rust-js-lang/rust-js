# 1065. A JS type a program vouches extends another is upcast safely

Status: Accepted: the user's decision (2026-11-02, rd622, option A of
three). Joins the vouching traits of [0225](0225-unknown-values.md)'s
`Defined`, `StructuredClone` and `JsonText` and webapi's `Record`
([0102](0102-js-and-webapi.md), amended).

Case ([0262](0262-when-rust-and-js-disagree.md)): none. The upcast is the
value itself, the same JS object, as `js::cast` of it was.

## Context

Upstream treats a native promise as a `Thenable` it instruments, as Flow's
subtyping lets it:

```js
const modulePromise: Thenable<T> = import(metadata.specifier);
modulePromise.then(value => {
  const fulfilledThenable: FulfilledThenable<mixed> = (modulePromise: any);
  fulfilledThenable.status = 'fulfilled';
  ..
```

React's port has `js::Promise<T>`, what `js::import_specifier`,
`js::promise::all` and `then` give, and shared's `Thenable`, a binding
type of its own, `struct Thenable(PhantomData<JsObject>)`, with `then`,
`status`, `value` and `reason`. Nothing said one is the other, so each
site wrote the cast:

```rust
// SAFETY: a promise is a thenable.
let thenable: &'static Thenable = unsafe { js::cast(Some(js::unknown(&promise))) };
```

ten times, in the Flight clients' bundler configs, `ReactFlightClient` and
`ReactFlightReplyClient`, and BundlerESM and BundlerNode were held on it:
an unchecked `unsafe` at each use, of a fact that's the types'.

## Decision

**A program says once that each value of a JS type is also one of
another, `unsafe impl<T> js::Extends<Thenable> for js::Promise<T> {}`,
and `js::upcast(value)` is then that value as the other, safely, its JS
the value itself:**

```rust
// SAFETY: a promise is a thenable, its `then` a promise's.
unsafe impl<T> js::Extends<Thenable> for js::Promise<T> {}

let thenable: &'static Thenable = js::upcast(js::promise::all(promises));
```

```js
const thenable = Promise.all(promises);
```

- **`pub unsafe trait Extends<U: ?Sized> {}`**, in the js crate beside
  `Defined`: unsafe to implement, as its `impl` vouches what JS has of a
  value, as `Record`'s and `JsonText`'s do. A reference to one extends
  what it does, so `js::upcast(&promise)` and `js::upcast(promise)` both
  are.
- **`js::upcast<U, T: Extends<U>>(value: T) -> &'static U`**, a binding
  of `link_name = "this"`, the value itself, as `js::unknown` is: no JS of
  its own, no compiler rule of its own. A JS object lives as long as JS
  holds it, so it's `'static`, as what a binding gives is.
- **The impl is the program's**, in the crate that owns `U` or `T`, as
  Rust's orphan rule asks: shared's `Thenable` says it of `js::Promise`.
  The js crate says it of no pair of its own yet.

## Why

- **One `unsafe`, where the fact is**: the claim "a promise is a
  thenable" is the types', so it's said once, beside the type, and each
  use is checked by rustc: a `js::upcast` to a type nothing vouches for is
  an error naming the missing impl.
- **It's the family a program knows**: `unsafe impl Defined`, `unsafe impl
  Record`, `unsafe impl JsonText` each vouch what JS has, and a safe API
  reads them.

## Alternatives

- **`impl From<js::Promise<T>> for &'static Thenable`**, the cast in its
  body: no new API, but a function of its own in the JS, `from(p) {
  return p; }`, unless it's inlined, and the same pattern written again by
  every program. Rejected.
- **A structural attribute, `#[rust_js::supertype_of(js::Promise)]`**,
  rust-js comparing `then`'s signatures: a checker of its own for what a
  binding's author says anyway. Rejected as more than the need.

## What holds it

- `test/js-any-values.test.ts`'s "a promise a program vouches is a
  thenable is upcast as it is" (the upcast prints nothing; a thenable
  instrumented through it settles as the promise does, in Node).
- `docs/guide/interop/types.md`'s example, held by `test/docs.test.ts`.
