# 0878. `instanceof` of a constructor held as a value is `js::instance_of`

Status: Accepted. Extends [0024](0024-web-crate.md)'s forms of a
binding's `link_name` by one, `"instanceof"`, beside `"instanceof X"`.

Case ([0262](0262-when-rust-and-js-disagree.md)): N: it's JS's own
operator, run as JS runs it, a constructor that's no function's
`TypeError` a panic, as any throw a binding lets through is.

## Context

React's ReactTaint tells binary data by its class:

```js
// This is the shared constructor of all typed arrays.
const TypedArrayConstructor = getPrototypeOf(Uint32Array.prototype).constructor;
..
} else if (
  value instanceof TypedArrayConstructor ||
  value instanceof DataView
) {
```

`"instanceof X"` tests one class a binding names, `this instanceof X`; a
constructor held as a value, `%TypedArray%`, which no global names, had
no way to be written, and the js crate had no `Uint32Array.prototype` and
no test of a `DataView`. The port's draft wrote `ArrayBuffer.isView(value)`,
which is true of another realm's typed array, where upstream's
`instanceof` is false: under jest, whose tests run in a realm of their
own, a wrong answer.

## Decision

**`js::instance_of(value, constructor)` is `value instanceof
constructor`, of any value on each side, as TypeScript's `instanceof`
takes `any`** (link form `"instanceof"`, `JsForm::InstanceOfGiven`). And
the js crate binds what upstream reads: each typed array's prototype,
`uint32_array::prototype()`, `Uint32Array.prototype`, and the test of its
one class no typed array derefs to, `data_view::is(value)`, `value
instanceof DataView`:

```rust
thread_local! {
    // This is the shared constructor of all typed arrays.
    #[cfg_attr(rust_js, rust_js::name = "TypedArrayConstructor")]
    static TYPED_ARRAY_CONSTRUCTOR: Option<&'static Unknown> = js::get(
        js::object::get_prototype_of(js::unknown(js::uint32_array::prototype())),
        "constructor",
    );
}

pub fn is_binary(value: Option<&Unknown>) -> bool {
    js::instance_of(value, TYPED_ARRAY_CONSTRUCTOR.with(|&constructor| constructor))
        || js::data_view::is(value)
}
```

```js
// This is the shared constructor of all typed arrays.
const TypedArrayConstructor = Object.getPrototypeOf(Uint32Array.prototype).constructor;

export function isBinary(value) {
  return value instanceof TypedArrayConstructor || value instanceof DataView;
}
```

- **One way each**: a class a binding names is tested by that binding,
  `"instanceof X"`, as the guide's `is_editor` and the js crate's
  `data_view::is` are; a constructor that's a value, given or read, by
  `js::instance_of`, as `js::get` reads a key that's a value where `"get
  x"` names one.
- **It runs nothing of Rust's**, as `"instanceof X"` doesn't (ADR 0867's
  `binding_runs_nothing`).

## Why

- **Upstream's test, as it answers**: `instanceof` is false of another
  realm's typed array, as React's is; `ArrayBuffer.isView` isn't.
- **TypeScript's types**: `any instanceof any`, the left any value and the
  right one a constructor at runtime, or JS's `TypeError`.

## Alternatives

- **`js::class::<T>()` of a binding type**, `js::class::<DataView>()`: a
  binding type isn't always a class, `ArrayBufferView` is TypeScript's
  interface, which no global names, so it would read a global that isn't
  there. The js crate's own test of `DataView` says what's a class.
- **A `js::Class` on the right**, TypeScript's `Function`: the
  constructor read from a prototype is `any`, a cast to make it one.
- **`ArrayBuffer.isView`**: a different answer of another realm's.

## What holds it

- `test/js-collections.test.ts`, "instanceof of a constructor value is
  js::instance_of": React's `TypedArrayConstructor` and `DataView`, run of
  typed arrays, a `DataView`, an `ArrayBuffer`, an object, text,
  `undefined` and another realm's typed array, which `ArrayBuffer.isView`
  calls a view; and a constructor of nothing, its `TypeError`.
- `test/builtins-coverage.test.ts`: the typed arrays' generator writes
  each `prototype()` and `data_view::is`.
- Mutations `instance-of-given-unknown` (`lower/bindings.ts`) and
  `instance-of-given-backwards` (`lower/binding_calls.ts`).
