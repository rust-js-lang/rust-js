# 1027. A held class's static is called on it, and whether the host has a class is `js::is_function` of it

Status: Accepted (rd235, a port gap; P1). Extends [0911](0911-a-bindings-class-is-the-global-typescript-names.md)'s
binding's class as a value and [0942](0942-whether-a-global-is-a-function-is-is-function.md)'s
`js::is_function`.

Case ([0262](0262-when-rust-and-js-disagree.md)): none. The static is read
of the class the program holds and called on it, as JS calls it, so its
`this` is that class; `typeof C === "function"` is what JS asks.

## Context

React DOM's ReactFiberConfigDOM keeps the `Promise` it found as it loads:

```js
const localPromise = typeof Promise === 'function' ? Promise : undefined;
..
callback => localPromise.resolve(null).then(callback).catch(handleErrorInNextTick)
```

A page that replaces `Promise` later still gets React's. The port had
neither half: `js::is_function(js::class::<Promise<()>>)` was refused, a
class's binding no global's binding, and a held `&Class` had no static,
so only the global `Promise.resolve` could be written.

## Decision

- **`js::is_function(js::class::<T>)` is `typeof C === "function"`** of
  the global class TypeScript names `T` (ADR 0911): `typeof Promise ===
  "function"`. As `js::is_defined` and `js::is_number` ask it, the
  question of a binding's global.
- **`class.call_static(f, (a, ..))` is `class.f(a, ..)`**: `f` a binding's
  static, `js::promise::resolve` of `Promise.resolve`, its member read of
  the class held and called on it, given the tuple's items, typed by
  `f`'s own signature (`js::Static`). What's no binding's static, a
  function of the crate's or a method, is refused.

```rust
thread_local! {
    static LOCAL_PROMISE: Option<&'static Class> =
        if js::is_function(js::class::<Promise<()>>) { Some(js::class::<Promise<()>>()) } else { None };
}

pub fn resolved() -> Option<Promise<Option<&'static Unknown>>> {
    LOCAL_PROMISE.with(|local_promise| {
        local_promise.map(|local_promise| local_promise.call_static(js::promise::resolve, (js::null::<&Unknown>(),)))
    })
}
```

```js
let localPromise;
if (typeof Promise === "function") {
  localPromise = Promise;
}

export function resolved() {
  return localPromise?.resolve(null);
}
```

## Why

- **The static is the binding's**: its signature types the call, and
  its name is the member, so nothing is written twice; a class held is
  any class, as JS's `localPromise` is.
- **One form**: a class held calls its statics so; a global's static is
  its binding called, `js::promise::resolve(..)`, as before.

## Consequences

- `Class::call_static`, `js::Static`, the binding form `static`.
- Held by `test/lowering-is-function.test.ts`'s "a class held as a value
  is asked whether it's a function, and its static called on it" (run:
  the global `Promise` replaced after load, React's still used) and "a
  class's static call of what's no binding's static is refused", and
  mutations `static-call-given-itself` (`binding_calls.ts`),
  `static-form-a-method` (`bindings/forms.ts`),
  `is-function-of-a-class-refused` (`mir/calls.ts`).
