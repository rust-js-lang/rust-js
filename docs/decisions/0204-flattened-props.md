# 0204. A flattened field's struct is its parent's props

Status: Accepted. Extends [0195](0195-rest-props.md) and [0196](0196-typescript-declarations.md);
amended by [0205](0205-flattened-chains.md), which chains them and gives a
name both have to the props.

## Context

TypeScript makes a component's props of other types', `AnchorProps &
ButtonLinkProps`: one object, every field of both, and `...props` what
the component doesn't take by name. Rust has no `&`: a struct names its
fields, and one holding another holds it as a field, nested, where JS's
props are flat. What a component takes besides was a `Rest` (ADR 0195),
an object Rust can't look into.

## Decision

**A props struct's field marked `#[rust_js::flatten]` holds a struct whose
fields are the component's own props, as serde's `#[serde(flatten)]` has
a struct's fields in its parent's JSON. It's a `Rest`, typed: what the
component takes apart besides its other fields.**

```rust
#[derive(Default)]
pub struct Anchor {
    pub href: Option<&'static str>,
    pub target: Option<&'static str>,
}

#[derive(Default)]
pub struct ButtonLinkProps<C> {
    pub size: Option<&'static str>,
    pub children: C,
    #[cfg_attr(rust_js, rust_js::flatten)]
    pub anchor: Anchor,
}

pub fn ButtonLink<C: Node>(ButtonLinkProps { size, children, anchor }: ButtonLinkProps<C>) -> Element { .. }

<ButtonLink size={Some("lg")} anchor={Anchor { href: Some("/learn"), ..Default::default() }} {..Default::default()}>
```

```jsx
export function ButtonLink({ size, children, ...anchor }) { .. }

<ButtonLink size="lg" href="/learn">
```

```ts
export interface ButtonLinkProps<C> extends Anchor {
  size?: string;
  children: C;
}
```

- **Taken apart, it's `...anchor`**, its fields read as any struct's,
  `anchor.target`, and spread as any struct is, `<a {...anchor}>`.
- **In JSX, one made there is its fields, each an attribute**; another,
  `{...anchor}`.
- **Declared, it's what the props extend**: its struct, or, one of
  another module, which TypeScript there can't see (ADR 0196), what a
  `Rest` is, `[prop: string]: unknown`.
- **What JS's props can't be is an error**: a struct with two rests, a
  `Rest` and a flattened field or two of these; a flattened field that
  isn't a struct of named fields, or is one within another; a field name
  both have; the field read, `p.anchor`, or taken apart anywhere but
  where the props are given; and a struct with one made anywhere but as
  `jsx!`'s props, or their `{..Default::default()}`, as it would be
  nested.

## Why

- **It's the props JS has**: one flat object, which TypeScript types as
  the two together, and a JS caller gives, `<ButtonLink href="/x">`.
- **It's Rust's own struct**, typed: each field is checked where it's
  given and read, where a `Rest`'s aren't.
- **It's tested**: a JSX test renders a component with a flattened
  struct, called by Rust and by `createElement`, and refuses each misuse;
  the declarations test checks TypeScript sees a flattened struct's
  fields as the component's own, and finds a wrong one.

## Costs

- **A Rust caller writes the struct**, `anchor={Anchor { .. }}`: `jsx!`
  doesn't know a component's fields, so it can't take `href` as one.
- **What a JS caller gives besides is in it too**, as TypeScript's `&`
  allows: `{...anchor}` passes it on.
- **No `Omit`**: a name both structs have is an error, so a flattened
  struct holds only what its parent doesn't. ADR 0205 gives it to the
  parent.
