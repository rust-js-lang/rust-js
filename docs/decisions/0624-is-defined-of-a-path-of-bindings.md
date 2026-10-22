# 0624. `js::is_defined` of a path of bindings asks each part

Status: Accepted. Extends [0597](0597-whether-the-host-has-a-global-is-is-defined.md).

Case ([0262](0262-when-rust-and-js-disagree.md)): A: what's asked of the
host, part by part, as JS asks it.

## Context

React's `shared/ExecutionEnvironment` asks whether the host has a DOM:

```js
export const canUseDOM = !!(
  typeof window !== 'undefined' &&
  typeof window.document !== 'undefined' &&
  typeof window.document.createElement !== 'undefined'
);
```

`js::is_defined(webapi::Window::document)` asks the first two, a method
of the one global its type is (ADR 0597). `createElement` is
`Document`'s method, and `js::is_defined(webapi::Document::create_element)`
asks the `document` global's, the one global of that type, not
`window.document`'s. Nothing said a path through what a global's member
gives.

## Decision

**`js::is_defined` of a tuple of bindings asks each part of the path they
make**, each step a method of what the one before gives, the first a
global's binding as ADR 0597 asks it:

```rust
thread_local! {
    #[cfg_attr(rust_js, rust_js::name = "canUseDOM")]
    pub static CAN_USE_DOM: bool = js::is_defined((Window::document, Document::create_element::<Div>));
}
```

```js
export const canUseDOM =
  typeof window !== 'undefined' &&
  typeof window.document !== 'undefined' &&
  typeof window.document.createElement !== 'undefined';
```

A step whose `self` isn't what the one before gives, or that's no
binding, is refused: "`js::is_defined` of a path of bindings where a step
isn't a method of what the one before gives". Decided with the user.

## Why

- **It's the path the program names**, typed: each step is checked
  against the one before by rustc's types, not spelled as text.
- **One question, one function**: `js::is_defined`, of a binding or of a
  path of them; a single binding is the path of one step it always was.

## Alternatives

- **Two asks, `is_defined(Window::document) && is_defined(Document::create_element)`**:
  asks the `document` global, not `window.document`. Rejected by the user.
- **A path as text, `is_defined!("window.document.createElement")`**: a
  second spelling of a global, unchecked.

## Consequences

- A generic method is named with its types, `create_element::<Div>`, as
  Rust names any generic function as a value; the JS asks the same.
- Upstream's `!!( .. )` isn't written: the `&&` of `typeof` tests is a
  boolean already, and Rust's `bool` needs no cast.

## What holds it

- `test/lowering-bindings.test.ts`, "js::is_defined of a path of
  bindings asks each part": a getter then a generic method, asked of a
  host with none, some and all of the path, and a step refused.
- Mutations `defined-path-unread`, `defined-path-step-unchecked`.
