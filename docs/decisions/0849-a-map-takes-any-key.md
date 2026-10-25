# 0849. A `js::Map` takes any key, as TypeScript's does

Status: Accepted. Amends [0699](0699-a-map-of-objects-is-a-weak-map.md),
whose `js::Map` took what a `WeakMap` keys alone. Amended by
[0884](0884-a-maps-value-is-any-value.md): its value is any value too, and
`get` of an `Option` tells a `None` from a missing key, by `has`.

Case ([0262](0262-when-rust-and-js-disagree.md)): N: it's JS's own
collection, its key the one `SameValueZero` says, as TypeScript's `Map` is.

## Context

React's cache keeps a primitive argument in a `Map`, `undefined`, `null`,
a string, a number:

```js
let primitiveCache = cacheNode.p; // Map<string | number | null | void | symbol | boolean, CacheNode<T>>
if (primitiveCache === null) {
  cacheNode.p = primitiveCache = new Map();
}
const primitiveNode = primitiveCache.get(arg);
```

`js::Map<K, V>` took `K: WeakKey`, an object, as ADR 0699 made it for a
`WeakMap`'s fallback: `map::new::<Option<&Unknown>, _>()` was E0277. A
`HashMap` keys by `Hash` and `Eq`, which no `&Unknown` has. A type
narrower than TypeScript's is a port's gap.

## Decision

**`js::Map<K, V>` is TypeScript's `Map<K, V>`, of any `K`: its `get`,
`set`, `has`, `delete`, `size` and `clear` its own; one of what a
`WeakMap` keys still derefs to a `WeakMap<K, V>`, as TypeScript's is
one.** A `WeakMap` keys what it holds weakly, an object, as before.

```rust
let map: &Map<Option<&'static Unknown>, u32> = js::map::new();
if !map.has(&key) {
    map.set(&key, i as u32);
}
let texts: &Map<str, f64> = js::map::new();
texts.set("a", 1.0);
```

```js
const map = new Map();
if (!map.has(key)) {
  map.set(key, i);
}
const texts = new Map();
texts.set("a", 1);
```

- **A key is JS's**: what `SameValueZero` says is one, `undefined` and
  `null` two, `0` and `-0` one, `NaN` one, an object by itself, text and
  a number by its value.
- **A map a program keeps of what Rust hashes is still a `HashMap`**, a
  `Map` in JS too, which keys a struct by its `Eq` (ADR 0059); a
  `js::Map` is JS's, to share with JS or to key what no `Hash` has, a
  value of any shape.

## Why

- **It's TypeScript's type**, `Map<K, V>` of any `K`, which upstream's
  primitive cache is.
- **A `HashMap` can't key a value of any shape**: JS's `===` isn't an
  `Eq`, `NaN` itself, and an object is no hash.

## Alternatives

- **A key trait of what JS keys by value or by itself**, primitives,
  `Option<&Unknown>`, a `WeakKey`: narrower than TypeScript's, a tuple's
  array keyed by itself as TypeScript's is, which JS's `Map` documents.
- **A second type, `js::PrimitiveMap`**: a second map of one JS
  collection.

## What holds it

- `test/js-builtins.test.ts`, "a Map takes any key, as TypeScript's
  does": each kind of key run, `undefined` and `null` apart, `0` and `-0`
  one, `NaN` itself, an object by itself, and a `Map<str, f64>`.
  `test/rest-parameters.test.ts`, React's primitive cache run against
  upstream's. ADR 0699's test, a `Map` as a `WeakMap`, as it was.
