# 0265. A component a thread_local! holds takes props of any lifetime

Status: Accepted. Extends [0213](0213-props-as-written.md) and
[0263](0263-with-router.md).

Case: A ([0262](0262-when-rust-and-js-disagree.md)).

## Context

A component `memo`, `lazy`, `forwardRef` or a library's function makes
is a `thread_local!`, typed with its props' `'static` form:

```rust
thread_local! {
    pub static DocsPageFooter: MemoExoticComponent<DocsPageFooterProps<'static>> = memo_with(PageFooter, areEqual);
}
```

So Rust held every prop a caller gave it to `'static`: react.dev's Page
couldn't give Seo the image it makes, `&format!(..)`, nor DocsPageFooter
the routes it borrows, without leaking them. JS frees nothing a render
still reads, so no program can tell a borrow from a `'static` one.

## Decision

- **Each props struct a component takes says its `'static` form**: `jsx!`
  writes `impl ::react::Lifetimes for LabelProps<'a> { type Static =
  LabelProps<'static>; }` beside its companion (ADR 0213).
- **A `thread_local!`'s component is given props of any lifetime**: its
  companion calls `react::static_component`, whose props' `'static` form
  is the component's, and whose JSX is `component`'s.
- **An impl of a trait of types only runs nothing**, as one of no items
  doesn't: rust-js takes it, and writes nothing of it.

```rust
let text = format!("Hi {name}");
jsx! { <Memoized text={&text} /> }
```

```js
const text = `Hi ${name}`;
return <Memoized text={text} />;
```

## Why

- **It's how the JS is**: a component reads its props while it renders,
  and JS keeps them alive while it does.
- **It's tested**: a JSX test gives a `memo`'s component a string its
  caller made, and renders it; mutations keep the component's props
  `'static`, the struct's `'static` form its own lifetimes, say none, and
  refuse the impl.
