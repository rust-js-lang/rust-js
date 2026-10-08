# 0258. A promise's then, a module of a component, next/head and the Router

Status: Accepted. Extends [0102](0102-js-and-webapi.md) and
[0192](0192-next.md).

Case: N ([0262](0262-when-rust-and-js-disagree.md)).

## Context

react.dev's Search loads DocSearch's modal, whose module doesn't export
it by default, preconnects in the page's head, and goes to a result by
next/router's own router:

```tsx
const DocSearchModal = lazy(() =>
  import('@docsearch/react/modal').then((mod) => ({default: mod.DocSearchModal}))
);
<Head><link rel="preconnect" href={..} /></Head>
Router.push(itemUrl);
```

The builtins crate's `Promise` had no `then`, react's `Module` no way to
be made, and the next crate neither `next/head` nor the default router.

## Decision

- **`Promise::then_resolve(f)`** is `promise.then(f)`, a promise of what
  `f` makes, as ReScript's `Promise.thenResolve`; `f` gives no promise.
- **`Module::of(component)`** is `{ default: component }`, a module of it
  as its default, which `lazy` loads.
- **`next::head::Head`** is `next/head`'s, of its children.
- **`next::router::Router`** is `next/router`'s default export, a
  `NextRouter` used outside a component.

```rust
import_modal("@docsearch/react/modal").then_resolve(|r#mod| Module::of(doc_search_modal(r#mod)))
```

```js
import("@docsearch/react/modal").then((mod) => ({ default: mod.DocSearchModal }))
```

## Why

- **They're JS's, React's and Next.js's own**, as the site uses them.
- **They're tested**: a JSX test loads a module's named component as a
  module's default; the Next.js build test compiles a head and a push of
  the default router; the crates' packages list `next/head`.
