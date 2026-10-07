# 0234. A component of an element's props is an `ElementType`, rendered as a tag

Status: Accepted. Extends [0220](0220-jsx-tag-values.md).

## Context

react.dev's ExpandableCallout keeps an icon per callout in `variantMap`,
`Icon: IconWarning`, and renders the one it has:

```jsx
{variant.Icon && <variant.Icon className={cn("inline me-2 mb-1 text-lg", variant.textColor)} />}
```

Each icon is `memo<JSX.IntrinsicElements['svg']>`, and two take a `size`
too, which TypeScript lets one field hold, as each takes an `<svg>`'s
props. In Rust each icon's props are a type of its own, so no field held
them all; and `jsx!` read `<Icon ..>` of a closure's `Icon` as a
component's name, `Icon!`, which there's none of.

## Decision

**Props that take what a DOM element takes are `react::ElementProps`; a
`memo` component of them is a `Tag`; and `react::ElementType`, as
@types/react's, is a `Tag` that holds any of them, made by
`element_type`, which is the value itself. A closure's capitalized
bindings are tags in it, as a function's are.**

```rust
pub struct Variant {
    #[cfg_attr(rust_js, rust_js::name = "Icon")]
    pub icon: Option<ElementType>,
}
Variant { icon: Some(element_type(&IconWarning)), .. }
variant.icon.map(|Icon| jsx! { <Icon className={..} /> })
```

```jsx
{ Icon: IconWarning, .. }
variant.Icon != null ? <variant.Icon className={..} /> : undefined
```

- **`SVGAttributes` are `ElementProps`**, and a component's that flatten
  them and have nothing else it must be given say so by hand, `impl
  ElementProps for IconCanaryProps {}`, as a `Tag` is said.
- **It's rendered as a tag is** (ADR 0220), `react::tag(Icon)` with an
  element's attributes, which its component takes.
- **A closure's parameters and `let`s inside JSX are its own**, `|Icon|`
  of a child's `map`, as a function's are.

## Why

- **It's the JS a person writes**: the icon itself in the table, and
  `<variant.Icon .. />` where it's rendered.
- **It's checked as far as Rust can**: only a `Tag` is an `ElementType`,
  and a `memo` component only when its props take an element's.
- **It's tested**: a JSX test holds two `memo` icons of different props,
  one flattening `SVGAttributes` beside a `size`, in one
  `Option<ElementType>` field, and renders each, and none.

## Costs

- **`ElementProps` is said, not proved**: rustc doesn't see that what's
  beside the flattened attributes is optional.
