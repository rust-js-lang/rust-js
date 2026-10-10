# 0226. A component looks inside its children as React's `Children` does

Status: Accepted. Extends [0214](0214-untagged-enums.md) and
[0043](0043-react-versions.md).

Case: N ([0262](0262-when-rust-and-js-disagree.md)).

## Context

react.dev's MDX components read their children: a `CodeDiagram` keeps its
`<img>`s apart from its code, an `ExpandableExample` takes its title from
its first child's `props`, a `Link` clones each `inlineCode` child with
`isLink`. They do it with React's own API:

```jsx
const content = Children.toArray(children).map((child) =>
  child.type?.mdxName === "pre" ? <CodeBlock key={child.key} {...child.props} /> : child,
);
if (isValidElement(children)) message = children.props.children;
return cloneElement(child, { isLink: true });
```

The `react` crate left `Children`, `isValidElement` and `cloneElement` out,
as legacy (ADR 0043), and `kind_of` told only text from any other node, so
11 of the components still to port couldn't be written, and TerminalBlock
bound `isValidElement` itself.

## Decision

**`react::children` is React's `Children`, `to_array` and `for_each`, each
child a `Child`: text, a number, a `ReactElement`, or another node. A
`ReactElement` is told by `isValidElement`, and has its `type`, `props` and
`key`; `clone_element` is `cloneElement`. `kind_of` tells an element too.**

```rust
for child in children::to_array(&children) {
    match child {
        Child::Element(element) if mdx_name(element.r#type()) == Some("pre") => ..,
        Child::Text(text) => ..,
        _ => ..,
    }
}
if let NodeKind::Element(element) = kind_of(&children) {
    message = js::get(element.props(), "children");
}
clone_element(element, InlineCodeProps { is_link: true })
```

```js
for (const child of Children.toArray(children)) {
  if (isValidElement(child)) { .. } else if (typeof child === "string") { .. }
}
if (isValidElement(children)) { message = children.props.children; }
cloneElement(element, { isLink: true })
```

- **An untagged enum's variant may be told by a function** its payload's
  type names, `#[rust_js::test = "react#isValidElement"]` on a JS object
  type, where no class says what it is: `isValidElement(v)` where a class's
  is `v instanceof C`. It's an object, so an object's test leaves it out, as
  it does an array; and a module that matches one imports the function.
- **`Child` and `NodeKind` are untagged enums** of what React's children
  are: `Children.toArray` leaves out `null`, `undefined` and booleans, so a
  `Child` is text, a number, an element, or `Other`, a portal say; a node
  of `kind_of` is text, an element, a `List` of nodes, as JSX gives several
  children, or `Other`. A list is the children themselves, not
  `Children.toArray`'s keyed copy: react.dev's ExpandableExample takes its
  first child as its title and renders `&items[1..]`, `children.slice(1)`.
  Each kind is a node, and a slice of nodes, and a `js::Unknown`, which a
  child's `props.children` is, as JSX renders anything. (Amended: there was
  no `List`, and an `Unknown` didn't render.)
- **What an element holds is a `js::Unknown`** (ADR 0225), its `type` and
  `props`, as @types/react's `ReactElement` has them `any`: a component's
  own properties, react.dev's `mdxName`, and a child's props are the
  site's, read with `js::get` or a binding of its own. Its props are of a
  type, `ReactElement<P = Unknown>`, as @types/react's `ReactElement<P =
  unknown>`: a child's, as a component's,
  `ReactElement::<ItemProps>::unchecked_from(e).props().title`, which is
  `child.props.title`, unchecked, as `isValidElement<P>` is; and
  `clone_element` keeps them. (Amended: they were an `Unknown` only.)
- **A `bigint` and a promise of a node are nodes too**, from React 19, as
  its `ReactNode` has them: an `i64`, `u64`, `i128` or `u128`, rust-js's
  `BigInt`s (ADR 0086), shown as their digits, and a `js::Promise<T>` of a
  node, which suspends until it settles. Gated `#[cfg(react = "19.0")]`
  (ADR 0043). (Amended.)
- **`clone_element(element, props)` takes a struct of the props it sets**,
  as `cloneElement`'s object, and makes a `ReactElement`, as @types/react
  types it: a child as any other, `Child::Element(clone_element(..))`, as
  react.dev's Link maps its children to a clone or the child itself.
  `element.element()` is it as what JSX makes, a component's result.
  (Amended: it made an `Element`, which no `Child` holds.)
- **Its `.d.ts` is @types/react's `ReactElement`.**

## Why

- **It's React's API, by its names**, as the TypeScript it replaces calls
  it, and it's what React documents for this, if as legacy.
- **The test is the language's**: a `match` on what a value is, as
  `typeof` and `instanceof` already are, not a cast after a check.
- **It's tested**: a JSX test lists a component's children by kind, text,
  a number, an `<img>` with its key, and a component, `null` and `true`
  left out; clones its one element; reads an `<h4>`'s `id`; and its JS is
  `Children.toArray`, `isValidElement` and `cloneElement`. React's coverage
  test counts the three bound.

## Costs

- **What an element holds is untyped**, as it is in TypeScript: a wrong
  prop's name is `undefined`, not an error.
- **A breaking change**: `NodeKind` has an `Element` variant, so a `match`
  of it needs an arm for one.

## Since

- **A list of a component's children and more** (2026-10-10):
  `Child::element(e)` is an element as a child, the element itself, so
  `[children::to_array(children), vec![Child::element(end)]].concat()` is
  `[...Children.toArray(children), end]`, as react.dev's
  SandpackWithHTMLOutput adds its sandbox's files; and `to_array` takes a
  node behind a reference, `&'static dyn ReactNode`'s, whose children
  live as long. A JSX test renders a component's children and one more.
