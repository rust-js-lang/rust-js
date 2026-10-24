# 0776. `js::get`, `js::has`, `js::set` and `js::delete` take any value

Status: Accepted. Amends [0225](0225-unknown-values.md), whose `get`, `has`
and `set` took a present value, `&Unknown`, and [0697](0697-delete-is-jss-delete.md),
whose `delete` did. Follows [0721](0721-strict-equals-is-jss-triple-equals.md)'s
`strict_equals`, whose operands are any value, `impl IntoUnknown`.

Case: C ([0262](0262-when-rust-and-js-disagree.md)): results changed. A
port had to guard what JS throws of, so a program that relies on the
throw, as React's tests do of its message, behaved otherwise.

## Context

React's `lazy` writes `'default' in moduleObject` and `moduleObject.default`
of what a loader resolved to, which may be `undefined`; `forwardRef` and
`memo` write `render.displayName = name` of a `render` that may be
`undefined`. In JS each throws a `TypeError` of `undefined` or `null`,
`Cannot use 'in' operator to search for 'default' in undefined`, and React's
tests check that message. `js::get`, `js::has` and `js::set` took a
`&Unknown`, a value that's never `undefined` nor `null`, so the port wrote
`module_object.is_some() &&` and `module_object.and_then(..)`, which
answer where JS throws: a behaviour difference, and Rust bent to get there.

TypeScript reads a property of an `any`, which `undefined` may be.

## Decision

**`js::get(value, key)`, `js::has(value, key)`, `js::set(value, key, to)`
and `js::delete(value, key)` take any value, `value: impl IntoUnknown`, as
`js::strict_equals` does: an `&Unknown`, an `Option<&Unknown>`, a
`&JsObject`, any value JS has. Each is JS's plain operator, `value.key`,
`"key" in value`, `value.key = to`, `delete value.key`, so of `undefined`
or `null`, a `None`, it throws JS's `TypeError`, a panic `catch_unwind`
catches (ADR 0710).** The JS of an existing caller is what it was.

```rust
pub fn default_of(module_object: Option<&Unknown>) -> Option<&'static Unknown> {
    js::get(module_object, "default")
}

pub fn has_default(module_object: Option<&Unknown>) -> bool {
    js::has(module_object, "default")
}

pub fn name(render: Option<&Unknown>, name: &str) {
    js::set(render, "displayName", name);
}
```

```js
export function defaultOf(moduleObject) {
  return moduleObject.default;
}

export function hasDefault(moduleObject) {
  return "default" in moduleObject;
}

export function name(render, name) {
  render.displayName = name;
}
```

## Why

- **It's JS's meaning**, which a binding of JS's own operator asks for:
  of `undefined`, the `TypeError` JS throws, its message JS's.
- **One signature for an operator of any value**: `strict_equals`,
  `get`, `has`, `set` and `delete` each take `impl IntoUnknown`, what
  TypeScript's `any` is given.
- **Nothing is bent**: `js::has(module_object, "default")` is upstream's
  `'default' in moduleObject`, with no guard upstream hasn't.

## Rejected

- **An `Option<&Unknown>` parameter**: a `&JsObject` or a `Some(&value)`
  would need converting, `Some(js::unknown(..))`, where `impl
  IntoUnknown` takes each as it is.
- **A second binding of each, of an `Option`**: two ways to read one
  property.

## Consequences

- A `None` given is no longer refused by rustc; it's JS's throw. A port
  that means "a property, if there's a value" writes Rust's
  `value.and_then(|v| js::get(v, key))`, `value?.key`.
- Held: `test/js-object.test.ts`, "get, has and set take any value, and
  throw of nothing as JS does", runs each of `undefined`, `null` and an
  object, and a `catch_unwind` of the throw; every other caller's JS is
  as it was (the lowering, JSX, docs and snapshot tests). No compiler rule
  changed, only the js crate's signatures, which rustc checks, so there's
  no mutation of one: the test fails to compile without them.
