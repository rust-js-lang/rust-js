# 0220. A JSX tag can be a value, named by a capitalized local

Status: Accepted. Extends [0040](0040-jsx.md) and [0213](0213-props-as-written.md).

## Context

JSX reads a capitalized variable as what to render, and a string there is a
DOM element's tag: react.dev's `Heading` is a `<div>`, or the `<h1>` to
`<h5>` its `as` prop names, with every other prop spread onto it:

```jsx
function Heading({ as: Comp = "div", className, children, id, ...props }, ref) {
  return <Comp id={id} {...props} ref={ref} className={cn("mdx-heading", className)}>..</Comp>;
}
```

`jsx!` had no such tag. A capitalized name was a component, built through
its props companion, `Comp!`, which a local doesn't have; and a spread had
to come last, where the original's `className` comes after it.

## Decision

**A fieldless enum whose variants are named as tags is a `react::Tag`, and
`jsx!` reads a tag that names a capitalized parameter or `let` of its
function as an element of it, which takes what a DOM element takes, a
spread among its attributes in their order:**

```rust
#[derive(Clone, Copy, Default)]
pub enum As {
    #[cfg_attr(rust_js, rust_js::name = "h1")]
    H1,
    #[default]
    #[cfg_attr(rust_js, rust_js::name = "div")]
    Div,
}
impl react::Tag for As {}

fn Heading(HeadingProps { r#as: Comp, id, children, rest, .. }: HeadingProps<..>) -> Element {
    jsx! { <Comp id={id} {...rest} className={..}>{children}</Comp> }
}
```

```jsx
function Heading({ as: Comp = "div", id, children, ...rest }) {
  return <Comp id={id} {...rest} className={..}>{children}</Comp>;
}
```

- **The names are the function's own**, its parameters' and `let`s'
  bindings that start with an uppercase letter, a field's pattern
  `r#as: Comp` among them; a function inside has its own. The formatter
  reads them as `jsx!` does.
- **It's `react::tag(Comp)`**, then the attributes a DOM element's are,
  which rust-js writes as JSX's `<Comp ..>`: a tag that's a variable.
- **A local given only a spread, `<Comp {...rest} />`, is a component's
  call, as before**, `let Selected = ui::Empty;` one: a `Tag` is a
  `Component` of any props too, so a tag takes them so as well.
- **A DOM element's spread may come before attributes**, as JSX's may,
  each written where it is, so what comes later wins, as in JSX. A
  component's spread still comes last, its named props from `{..base}`
  (ADR 0213).

## Why

- **It's the JSX the original writes**, `{ as: Comp = "div" }` and
  `<Comp ..>`, with the props it spreads where it spreads them.
- **It's checked**: `Comp` is a `Tag`, so not any value, and what it takes
  is a DOM element's attributes, each of its type.
- **It's tested**: a JSX test renders a `Heading` of a prop's tag, its
  default and a caller's, and of a `let`, and checks the JSX; the
  formatter lays out a tag's attributes after its spread; a component's
  attribute after a spread is still refused.

## Costs

- **A capitalized local hides a component of its name** in that
  function's JSX.
- **What a tag takes isn't narrowed to its element's**: an `<h1>` and a
  `<div>` take the same attributes, as React's own types give them.
