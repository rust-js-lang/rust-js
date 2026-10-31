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

## Amendment: a held class's constructor (2026-10-31)

React's componentFrameCache and React DOM's instance maps hold `const
PossiblyWeakMap = typeof WeakMap === 'function' ? WeakMap : Map;` and
make `new PossiblyWeakMap()` of it. `class.call_static(js::weak_map::new,
())` was refused, "not a binding's static method", so the port wrote
`unsafe { js::cast(Some(js::new(class, ()))) }`: a value of any shape
cast to what it makes.

**`Class::call_static` takes a constructor's binding too**, one whose
link name is `new C`, `js::weak_map::new`'s `new WeakMap`: `new
PossiblyWeakMap()`, of the class held, what it makes typed by the
binding's signature, as a static's result is. One form for what a class
held makes or gives: its binding names it. A trailing `None` given an
optional parameter of the binding's, a static's or a constructor's, is no
argument, as the binding's own call leaves it (ADR 0443):
`class.call_static(js::map::new, (None,))` is `new PossiblyWeakMap()`.

`js::new(value, arguments)` stays what it is, `new` of a class the
program makes, `Class<fn(A) -> T>`, typed, or of a value of any shape,
which makes one; a class held of a binding's is made by the binding's
constructor, so the guide's `AbortControllerLocal` is
`class.call_static(webapi::abort_controller::new, ())`, no cast.

Case: none ([0262](0262-when-rust-and-js-disagree.md)'s first): `new` of
the class held, as upstream's. Held by
`test/lowering-is-function.test.ts`'s "a held class is constructed by its
constructor's binding", which runs it with WeakMap and, loaded again
where the host has none, with Map; mutations `static-constructor-refused`,
`static-constructor-called` and `static-none-given`.

## Amendment: `call_static` is the one way to make a class held (2026-10-31)

Two forms made a class held, an untyped `&Class`: `js::new(class, ())`,
which made a value of any shape a program then cast, and
`class.call_static(webapi::abort_controller::new, ())`, typed by the
constructor's binding. One way, the coordinator's decision: **a class held
is made by its constructor's binding, `call_static`**. The js crate no
longer implements `Construct` of an untyped `Class`, so rustc refuses
`js::new(class, ..)` of one, its note naming `call_static`; and
`js::new(js::unknown(class), ())` of one is refused naming it too
(`recognition::is_untyped_class`). `js::new` keeps its other uses: a class
`Class::of` makes, typed, and a value of any shape. A class the program
knows is made by its constructor, `abort_controller::new()`, or the
crate's `Shim::new()`.

Case: N ([0262](0262-when-rust-and-js-disagree.md)), one form for one
job. Held by `test/js-constructors.test.ts`' "new of a class held is
refused, naming its constructor's binding" and "a constructor held as a
value of any shape is refused", `test/webapi-bindings.test.ts`' "a
binding's class is the global class TypeScript names", migrated; mutations
`held-class-as-unknown-named-new` (`classes.ts`) and `untyped-class-never`
(`recognition/types.ts`). The port's React DOM `replayChangeEvent` writes
`event_ctr.call_static(event::new, ("click", Some(&init)))`.
