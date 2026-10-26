# 0911. A binding's class is the global class TypeScript names

Status: Accepted. Extends [0761](0761-a-rust-js-class-is-a-constructor-function-and-its-prototype.md)'s
`js::class::<T>()`, which took a `#[rust_js::class]` struct only, to a
binding's type, by [0597](0597-whether-the-host-has-a-global-is-is-defined.md)'s
and [0891](0891-a-windows-constructor-is-the-windows.md)'s rule that a
class's constructor binding is the global scope's.

Case ([0262](0262-when-rust-and-js-disagree.md)): N. The class is the
value the program names, `AbortController`, which `new` makes one of as
JS's `new` does; nothing of Rust's differs.

## Context

React's `ReactFiberCacheComponent` keeps the host's `AbortController`, and
a shim where the host has none, as one value it constructs later:

```js
const AbortControllerLocal: typeof AbortController =
  typeof AbortController !== 'undefined'
    ? AbortController
    : function AbortControllerShim() { .. };
..
controller: new AbortControllerLocal(),
```

The port wrote it as it reads, a `&'static js::Class` of either class:

```rust
static ABORT_CONTROLLER_LOCAL: &'static Class = if js::is_defined(webapi::abort_controller::new) {
    js::class::<AbortController>()
} else {
    js::class::<AbortControllerShim>()
};
```

and rust-js refused it: "does not support `js::class::<T>()` of a `T`
that's no `#[rust_js::class]` struct yet". `js::class` gave the crate's
own class only, though webapi's `AbortController` is a class too, the
global one `webapi::abort_controller::new` constructs.

## Decision

**`js::class::<T>()` of a binding's JS object type is the global class
TypeScript names it: `js::class::<AbortController>()` is
`AbortController`.** Its class is the one `instanceof` tests a `T` by (an
untagged enum's variant of one, ADR 0214), `untagged.rs`'s `class_value`,
where the type's `#[rust_js::types]` names that class, as each of
webapi's does, `#[rust_js::types = "AbortController"]`, and the js
crate's `Map<K, V>` does. It's the global scope's, as its constructor
binding, `abort_controller::new`, is (ADR 0891), and as `js::is_defined`
of that binding asks of it.

```js
const AbortControllerLocal =
  typeof AbortController !== "undefined" ? AbortController : AbortControllerShim;

export function controller() {
  const controller = new AbortControllerLocal();
```

- **`js::new` of either is JS's `new`**, of the `&Class` as a value of
  any shape, `js::new(js::unknown(class), ())`, as it was of the crate's
  class (ADR 0695); what it makes is cast to what the program knows it
  is, as any value of any shape is.
- **What TypeScript names no class is refused**, as before: a `JsError`,
  what JS threw, of no one class; a `JsObject` or an `Unknown`,
  TypeScript's `object` and `unknown`; a type TypeScript doesn't name, a
  `.d.ts`'s `any`, which may be anything at run time. The error says what
  names one.

## Why

- **It's the program's own Rust**: one `js::class::<T>()` of each class,
  the crate's or a binding's, as `instanceof` and `js::is_defined` already
  take either; no constructor-as-a-value type per class.
- **One authority for a binding's class**: the class `instanceof` tests,
  which an untagged enum asks too, not a second reading of a binding's
  name.
- **Only where it's known to be a class**: a binding type whose
  TypeScript type is a class by that name has the global TypeScript
  declares, `declare var AbortController: { new(): AbortController }`.
  One TypeScript doesn't name, `JsError`'s, would be a `ReferenceError`.

## Alternatives

- **A class from its constructor binding**, `js::class_of(abort_controller::new)`:
  a second way to name a class, beside `js::class::<T>()`, and a
  function item read for its `link_name`.
- **Every binding's JS object type**, by its name: `js::class::<JsError>()`
  would be `JsError`, which no host has, a crash where the refusal was.
- **A window's class**, `window.AbortController`: that's a method's,
  `Window::new_abort_controller` (ADR 0891); the global's is the
  type's, as its module's `new` is.

## Consequences

- A class of a binding crate's own, `#[rust_js::types = "EditorView"]`,
  is the global `EditorView`; one its module exports is the next program's
  to need.
- An untagged enum's variant of a type TypeScript names no class, a
  `JsError`'s, is still tested `instanceof JsError`, which throws: issue
  #303 holds it.

## What holds it

- `test/webapi-bindings.test.ts`, "a binding's class is the global class
  TypeScript names": React's `AbortControllerLocal`, constructed in Bun
  where the host has `AbortController` and where it hasn't, its shim; and
  `JsError`, `JsObject` and `Unknown` refused.
- Mutations `binding-class-refused`, `binding-class-any-name`.
