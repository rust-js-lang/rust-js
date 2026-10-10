# 0353. A component's `getInitialProps` is set on it

Status: Accepted. Part of [0346](0346-next-coverage.md).

Case: N ([0262](0262-when-rust-and-js-disagree.md)).

## Context

Next.js reads a page's, a custom app's and a custom document's initial
props from a function set on the component, `MyDocument.getInitialProps =
async (ctx) => ..`, as `NextComponentType` types it. A Rust function is no
object to set a property of: `js::set` takes a JS value, which a function
item isn't, and an exported `getInitialProps` beside it Next.js never reads.

## Decision

**`next::set_get_initial_props(component, get)` is `component.getInitialProps
= get`, written in a `js::on_load!`, where JS sets it:**

```rust
js::on_load! {
    next::set_get_initial_props(MyDocument, initial);
}
```

```js
MyDocument.getInitialProps = initial;
```

- **It's a binding's setter** of its `this`, `link_name = "set
  getInitialProps"`, as a JS object's setters are bound: no new form.
- **`get` takes its context by reference**, `&'static DocumentContext`,
  `AppContext` or `NextPageContext`, the component's.
- **`App.getInitialProps` and `Document.getInitialProps`**, Next.js's own,
  are `next::app::app::get_initial_props` and
  `next::document::document::get_initial_props`, a class's statics in a
  module of its name, as webapi's are, imported as `App` and `Document`
  (ADR 0355).

## Why

- **It's the JS a custom document writes**, and what Next.js reads.

## Consequences

- `test/next.test.ts`'s document sets one, of Next.js's own, and the build
  renders each page through it.
