# 0201. A component takes no dictionary, and an update makes no default it replaces

Status: Accepted. Extends [0199](0199-components-take-no-drops.md) and
[0192](0192-next.md).

Case: A, D ([0262](0262-when-rust-and-js-disagree.md)).

## Context

react.dev's `ButtonLink` gives its children, a type parameter, to
next/link's `Link`, the rest of its props from a base,
`{..Default::default()}`. Rust's derived `Default` of `LinkProps<C>` asks
`C: Default`, so `ButtonLink<C: Node + Default>`, and rust-js:

- **Gave the component a dictionary**, `ButtonLink(props, CDefault)`, as
  any function a bound asks one of (ADR 0049), which React never gives:
  a JS caller's render threw, and a Rust caller's `<ButtonLink>` was an
  error, a function with a dictionary being no component.
- **Made the base whole**, its children's `CDefault.default()` too, then
  kept it, `const base = {..}`, and gave `Link` each field the update
  didn't name, `replace={base.replace}`.

## Decision

**A component takes no dictionary: React calls it with its props alone.
Its bounds are Rust's only, and a body that would use one is an error.
And an update's derived `Default` base makes no default of a field the
update gives where making it does nothing: each field's of a constant, `[]`
and the like, and a `Node`'s, which is std's or React's.**

```rust
pub fn ButtonLink<C: Node + Default>(ButtonLinkProps { href, children, .. }: ButtonLinkProps<C>) -> Element {
    jsx! { <Link href={href} {..Default::default()}>{children}</Link> }
}
```

```jsx
export function ButtonLink({ href, children }) {
  return <Link href={href}>{children}</Link>;
}
```

```text
error: rust-js does not support a component's `<C as std::default::Default>` yet: React gives a component no dictionary
```

- **react's `Node` is sealed**, `rust_js::jsx_node`: its types are
  react's, whose `Default`, where one has it, makes an empty `Element`,
  text, number, `Option`, `Vec` or tuple of them, and nothing else.
- **Each default the update reads is made in its place**, in the order
  the base would make them: `Counter { n: 1, ..Default::default() }` is
  `{ n: 1, label: "", items: [] }`, where it was a `const base` read for
  `base.items`.
- **A default it replaces that does something**, a hand-written
  `Default`'s, is still made, as Rust makes it, with the rest: a base,
  kept as before.

## Why

- **It's what React does**: a component is called with its props, so it
  takes them only, and a type parameter's default, which only Rust's
  derive asked for, is never made, as the update replaces it.
- **It's exact**: a default not made is one that does nothing, and each
  one made is made in Rust's order, so no program can tell. A JSX test builds a generic component that gives its
  children on with a base, called by Rust and by `createElement`, with no
  dictionary and no base, and refuses one whose body asks for one.

## Costs

- **A component generic over its children that gives them on with a base
  says `C: Node + Default`**, Rust's derive's bound, where `C: Node` would
  do for a JS reader.
- **A user's type can't be a `Node`.**
