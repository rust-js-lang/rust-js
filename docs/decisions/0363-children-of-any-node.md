# 0363. Children of any node are given by reference

Status: Accepted. Extends [0213](0213-props-as-written.md)'s companions and
[0239](0239-local-components.md)'s components of a function type.

Case: N ([0262](0262-when-rust-and-js-disagree.md)).

## Context

react.dev's 404 and 500 pages take components out of the MDX table and
render them with JSX's children:

```js
const {Intro, MaxWidth, p: P, a: A} = MDXComponents;
// ..
<Intro>
  <P>This page doesn’t exist.</P>
  <P>..</P>
</Intro>
```

A table of components in Rust is a struct of function pointers, and a
pointer takes one type of props: its children can't be the generic `C`
a component of the crate's takes. `&'static dyn ReactNode`, any node,
can be the type, as MDX gives anything, but JSX's children are a tuple
made right there, `(P, P)`, which no `'static` reference reaches. And
a constant of a component's function type, `const P: fn(..) = T.p;`,
wasn't a component JSX could name.

## Decision

**Props whose `children` are declared `&'a dyn ReactNode`, any node, as
TypeScript's `children: ReactNode`, or an `Option` of one, `children?`,
are given JSX's children by reference, `&(a, b)`, `Some(&(a, b))`; none
given is `&()`, or left out. A pointer to such a component is
`for<'a> fn(Props<'a>) -> JSX::Element`, and a constant of that type is
a component JSX names, its props built by their companion:**

```rust
pub struct IntroProps<'a> {
    pub children: Option<&'a dyn ReactNode>,
}
const Intro: for<'a> fn(IntroProps<'a>) -> JSX::Element = MDXComponents.Intro;
jsx! { <Intro><P>{"This page doesn’t exist."}</P></Intro> }
```

```jsx
const { Intro, MaxWidth, p: P, a: A } = MDXComponents;
<Intro>
  <P>This page doesn’t exist.</P>
</Intro>
```

- **The children are what's referred to**: `&(a, b)` is each a child,
  as JSX writes them, a test among them `test && <el />` (ADR 0235), not
  one array.
- **Told by how the field is written**, as a companion knows its fields
  (ADR 0213): a reference to `dyn ReactNode`, or an `Option` of one.
  Generic children, `C`, are given as they were.
- **A constant's props are another component's**, wherever it is, so
  they're built by their companion, which is imported with them.

## Why

- **It's TypeScript's `ReactNode`**: one function, and one pointer to
  it, takes children of any type, as a component does in JS.
- **It's the JSX of the original**: the table's components, taken out,
  rendered with their children, nothing wrapped.
- **It's tested**: a JSX test renders a table's component by a constant
  in another module, with children of any node and optional ones, a test
  among them; mutations give the children as they are, unspread, read
  through a reborrow, and leave the constant no component.

## Alternatives

- **Import each component instead of the table's**: the 404 page's JS
  unlike the original, and `MaxWidth`, written only in the table, made a
  function of its own.
- **Children a type that converts**, `Into`: a generic component's
  children would have no type to infer.

## Consequences

- react.dev's MDX `Intro`, `Link` and the table's own callouts and
  elements take `&'a dyn ReactNode` children; their JS is unchanged, and
  their declarations say `children?: ReactNode` where the original does.
