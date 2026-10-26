# 0677. A weak collection's key is what `js::Kind` holds

Status: Accepted. Amends [0283](0283-js-crate-covers-typescripts-es-library.md):
a weak collection is typed as TypeScript types it. Amends
[0225](0225-unknown-values.md): `Kind::Object` holds a `&JsObject`. Amended by
[0910](0910-an-object-or-an-array-is-one-value-kind-object.md): an
object or an array is one key, `kind.object()`'s `&JsObject`.

Case ([0262](0262-when-rust-and-js-disagree.md)): N. Each key is the
value itself, and only an object or a symbol is one, as JS's weak
collections take; no JS changes but what's no longer written.

## Context

React's `shared/ReactSerializationErrors.js` keys a `WeakMap` by any
object, the props and the children an element was made of:

```js
export const jsxPropsParents: WeakMap<any, any> = new WeakMap();
..
} else if (__DEV__ && jsxPropsParents.has(objectOrArray)) {
```

`js::WeakMap<K, V>` needs `K: WeakKey`, and `Kind::Object(o)` gave a
`&Unknown`, which isn't one: a string or a number may be an `Unknown`, and
`WeakMap.prototype.set` throws of one. A port cast it, `unsafe {
js::cast::<&JsObject>(..) }`, which printed `const key = objectOrArray;`,
and its `.d.ts` said `export const jsxPropsParents: any;`, as nothing said
how TypeScript types a `WeakMap`. TypeScript types it `WeakMap<WeakKey,
V>`, `WeakKey` an object or a symbol.

## Decision

**What `js::Kind` holds of an object, an array, a function and a symbol
is each a `WeakKey`, and a weak collection is declared as TypeScript's.**

```rust
thread_local! {
    pub static PROPS_PARENTS: &'static WeakMap<JsObject, &'static Unknown> = js::weak_map::new();
}

match js::classify(value) {
    Kind::Object(props) => PROPS_PARENTS.with(|parents| parents.set(props, parent)),
    ..
}
```
```js
export const propsParents = new WeakMap();
..
propsParents.set(value, parent);
```
```ts
export const propsParents: WeakMap<object, unknown>;
```

- **`Kind::Object` holds a `&JsObject`**, TypeScript's `object`, which
  derefs to the `Unknown` it is: `js::get(object, key)`, `object::keys`,
  a function of an `&Unknown` take it as they did. Its `Deref` is the
  object itself, as every JS type's is (ADR 0024); `JsObject` is
  `Defined`, and `js::concat_text` of one is a `Result`, as of an
  `Unknown`.
- **`[T]` and `dyn Fn()` are `WeakKey`s**, what `Kind::Array` and
  `Kind::Function` hold, each a JS object; `Symbol` was one.
- **`dyn WeakKey` is TypeScript's `WeakKey`**, a key of any of them that's
  sized: `WeakMap<dyn WeakKey, V>`, as `FinalizationRegistry`'s token is.
- **`WeakMap<K, V>`, `WeakSet<T>`, `WeakRef<T>` and
  `FinalizationRegistry<T>` are declared as TypeScript's**,
  `#[rust_js::types = "WeakMap<K, V>"]`, each argument its Rust type's.

## Why

- **It's TypeScript's**: `typeof value === "object" && value !== null`
  narrows an `unknown` to `object`, a `WeakKey`, as `Kind::Object` does.
- **No cast**: a cast is a promise a program can get wrong, and printed
  a name of its own.
- **One type of any object**: `JsObject` already is one, what
  `object::from_entries` gives; a second, `js::Object`, would be two.

## Alternatives

- **`WeakKey` for `Unknown`**: a string is an `Unknown`, and
  `map.set("a", 1)` throws a `TypeError`. The trait is `unsafe` because
  it promises JS takes the key.
- **`has`, `get` and `delete` of any value**, which JS answers `false` and
  `undefined` of a string: TypeScript doesn't type them so, and a second
  way to key a map.
- **An `Object` type of `Kind`'s own**: two types of any object.

## Consequences

- **A `&JsObject` is declared `object`**, where it was `any`: a
  discriminated union's `otherwise` of one, `{ kind: "on" } | object`.
- A pattern that bound `Kind::Object(o)` as a `&Unknown` still reads it as
  one, through `Deref`; one generic over its argument, `js::truthy`, takes
  a `JsObject` as the `Defined` it is.
- An object or an array as one value, `Kind::Object(_) | Kind::Array(_)`,
  is no one key: `[T]`'s `WeakMap` is its own, `WeakMap<[T], V>`, as
  React's `jsxChildrenParents` is of arrays, and `dyn WeakKey` holds no
  slice, which isn't sized. (Amended by ADR 0910: `kind.object()` is
  one, a `&JsObject`.)

## What holds it

- `test/js-builtins.test.ts`, "a weak map is keyed by what js::Kind
  holds": each kind's key, run in Node, and the `.d.ts`.
- The mutation `js-object-itself-unknown` in
  `scripts/mutations/lower/recognition/types.ts`.
