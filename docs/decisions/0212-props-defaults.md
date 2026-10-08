# 0212. A props field's default is where JS takes them apart

Status: Accepted.

Case: N, D ([0262](0262-when-rust-and-js-disagree.md)).

## Context

A React component writes its props' defaults where it takes them apart,
react.dev's `ButtonLink({ type = 'primary', size = 'md', target = '_self',
...props })`, and its TypeScript caller may leave them out. In Rust a
props field was required, or an `Option` the body defaulted,
`size.unwrap_or(Size::Md)`, which was `const match = size ?? "md"`, and a
TypeScript caller saw the field optional and its type `Size | undefined`
nowhere but in the body.

## Decision

**A props field marked `#[rust_js::default]` takes its type's `Default`,
and `#[rust_js::default = "_self"]` that string, where JS takes the props
apart; its type is the field's own, not an `Option`.**

```rust
pub struct ButtonLinkProps<'a, C> {
    #[cfg_attr(rust_js, rust_js::default)]
    pub size: Size,
    #[cfg_attr(rust_js, rust_js::default = "_self")]
    pub target: &'a str,
}
```

```jsx
export function ButtonLink({ size = "md", target = "_self" }) {
```

```ts
export interface ButtonLinkProps<C> {
    size?: Size;
    target?: string;
}
```

- **The default is a literal**, a constant or an array or object of them,
  as `Default` gives a unit-only enum's, a number's, a string's or a
  `Vec`'s; one that's made, a `HashMap`'s `new Map()`, is an error that
  says to write it.
- **The literal said is of the field's type**: a string of a string's, a
  `bool` of a `bool`'s, `#[rust_js::default = true]`, as react.dev's
  Heading takes `isPageAnchor = true`, and a number of a number's, written
  as a literal of that type is. One of another type is an error; `= true`
  was read as no literal at all, and the type's `Default`, `false`, taken.
- **An object of literals is a `const` the attribute names**,
  `#[rust_js::default(DEFAULT_PARAMETERS)]` of a `const` beside the
  struct, which no attribute's literal can say, as react.dev's Search
  takes `searchParameters = {hitsPerPage: 30, ..}`. (Amended.)
- **A component's props with one are taken apart where they're given**,
  `fn ButtonLink(ButtonLinkProps { size, .. }: ButtonLinkProps)`: taken
  whole, a JS caller's missing field would have none, so that's an error.
- **Declared, it's optional**, as a JS caller may leave it out.

## Why

- **It's React's own**: where a component takes its props apart is where
  JS writes their defaults, and the body has the value, of its own type.
- **It's exact**: a Rust caller gives every field; a JS caller's missing
  one is the default, the same value Rust's `Default` is.
- **It's tested**: a JSX test renders a component with a type's default
  and a string's, given and left out by a JS caller, and refuses props
  taken whole and a default that's made; the declarations test checks the
  field optional, which TypeScript lets a caller leave out.

## Costs

- **A default other than a type's `Default` is a string's**, as an
  attribute holds no other value Rust can check.
