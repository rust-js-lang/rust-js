# 0699. A `Map` of what a `WeakMap` keys is a `WeakMap`, for a fallback

Status: Accepted. Amends [0283](0283-js-crate-covers-typescripts-es-library.md),
whose JS `Map` was Rust's `HashMap` alone, and extends
[0677](0677-a-weak-key-is-what-kind-holds.md)'s weak keys. Amended by
[0849](0849-a-map-takes-any-key.md): a `js::Map` takes any key, as
TypeScript's does.

Case ([0262](0262-when-rust-and-js-disagree.md)): N: each is JS's own
collection, keyed by the object itself.

## Context

React's `shared/ReactComponentStackFrame.js` caches a frame of each
component, weakly where the host can:

```js
if (__DEV__) {
  const PossiblyWeakMap = typeof WeakMap === 'function' ? WeakMap : Map;
  componentFrameCache = new PossiblyWeakMap<Function, string>();
}
```

TypeScript types it `WeakMap<K, V> | Map<K, V>`. The port kept a
`WeakMap` and left the fallback out, "no type is either": a `HashMap`
keys by `Hash` and `Eq`, which no JS function has, and the js crate had
no `Map`. The user's rule is to keep upstream's fallbacks and let a
program express them.

## Decision

**`js::Map<K, V>` is JS's `Map` of what a `WeakMap` keys, an object by
itself, held strongly, and it derefs to a `WeakMap<K, V>`, as
TypeScript's `Map<K, V>` is one: so one value is either.**

```rust
thread_local! {
    static COMPONENT_FRAME_CACHE: &'static WeakMap<dyn Fn() -> Option<&'static Unknown>, String> =
        if js::is_defined(js::weak_map::new::<JsObject, String>) {
            js::weak_map::new()
        } else {
            js::map::new()
        };
}
```
```js
const componentFrameCache = typeof WeakMap !== "undefined" ? new WeakMap() : new Map();
```

- **Its key is a `WeakKey`**: what Rust hashes, text or a number, is a
  `HashMap`'s, a `Map` in JS too (ADR 0283); what it can't, an object, a
  function, a symbol, is this one's, keyed by itself.
- **`get`, `set`, `has` and `delete` are a `WeakMap`'s**, through its
  `Deref`, which is the map itself; its own `size` and `clear` are bound.
- **A `.d.ts` says `Map<K, V>`** of it, `WeakMap<K, V>` of the value held
  as one.
- **The test is `js::is_defined`'s**, `typeof WeakMap !== "undefined"`
  (ADR 0597), of `weak_map::new` given its type arguments, as a generic
  function needs them to be named.

## Why

- **TypeScript's types**: a `Map<K, V>` is assignable to a `WeakMap<K,
  V>`, so the union upstream makes is a `WeakMap`, and Rust's `Deref` is
  how the crates write one type that is another (ADR 0024).
- **One map of each kind of key**: `HashMap` of what Rust hashes, `Map`
  of what JS keys by itself.

## Alternatives

- **An enum of both**: a `match` at each `get`, where JS calls one
  method.
- **A trait both implement, `&dyn`**: a dictionary at each call,
  where JS calls the method.
- **`HashMap` keyed by a pointer**: a pointer's address isn't JS's.

## Consequences

- A `Map`'s iteration, `keys`, `values`, `entries`, `forEach`, isn't
  bound yet: none needs it, and a `HashMap` iterates what Rust hashes.
- `is_defined` of a generic constructor names its type arguments,
  `weak_map::new::<JsObject, String>`, which no JS reads.

## What holds it

- `test/js-builtins.test.ts`, "a Map is a WeakMap where the host has
  none": the cache made of either, run in Node with a `WeakMap` and with
  none; a `Map`'s `size`, `delete` and `clear`; its `.d.ts`.
- `test/js-object.test.ts`, "React's component stack frame is found as
  upstream finds it": `componentFrameCache`.
