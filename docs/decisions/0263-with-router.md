# 0263. next/router's withRouter, and a flattened struct taken apart

Status: Accepted. Extends [0192](0192-next.md) and
[0204](0204-flattened-props.md).

Case: N ([0262](0262-when-rust-and-js-disagree.md)).

## Context

react.dev's Seo is made by next/router's `withRouter`, which renders it
with the props its users give, and the router:

```tsx
export const Seo = withRouter(({title, titleForTitleTag, image = '/images/og-default.png', router, children, isHomePage, searchOrder}: SeoProps & {router: Router}) => { .. });
```

The next crate had no `withRouter`, the react crate no type for a
component a library's function makes, and a flattened struct's field
could be bound whole, `...props`, but not taken apart.

## Decision

- **`next::router::with_router(component)`** is `withRouter`, in a
  `thread_local!`; `component` takes `WithRouterProps<P>`, the props its
  users give flattened beside `router`.
- **`react::ComponentValue<P>`** is @types/react's `ComponentType<P>`, a
  component a library's function made: in a `thread_local!`, a tag, as
  `memo`'s is.
- **A flattened struct taken apart in a pattern gives its fields to its
  parent's**, as JS holds them, where it's written:

```rust
thread_local! {
    pub static Located: ComponentValue<LocatedProps<'static>> =
        with_router(|WithRouterProps { props: LocatedProps { prefix }, router }| jsx! { <p>{prefix}{router.as_path()}</p> });
}
```

```js
export const Located = withRouter(({ prefix, router }) => (
  <p>
    {prefix}
    {router.asPath}
  </p>
));
```

A rest beside one is refused: it would hold the fields the flattened
struct's pattern leaves.

## Why

- **It's the JS a person writes**, and next/router's own function.
- **It's tested**: the Next.js build test compiles a component `withRouter`
  makes and renders it as a tag; a JSX test takes a flattened struct apart
  with the props'. Mutations refuse the pattern, put its fields last, and
  leave the component no props companion.
