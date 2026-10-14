# 0384. A ref a prop holds is @types/react's `Ref<T>`, and an element's props `DetailedHTMLProps`

Status: Accepted. Amends [0227](0227-react-type-names.md) and
[0253](0253-next-link-ref.md); extends [0204](0204-flattened-props.md) and
[0214](0214-untagged-enums.md).

Case: N ([0262](0262-when-rust-and-js-disagree.md)): a ref is the value
itself in JS, as it is in TypeScript.

## Context

react.dev's ExternalLink takes an `<a>`'s props, `ref` too:

```tsx
export function ExternalLink({href, target, children, ...props}: DetailedHTMLProps<AnchorHTMLAttributes<HTMLAnchorElement>, HTMLAnchorElement>) {
```

The react crate had `react::Ref`, a trait of what JSX's `ref` takes, which
a function's parameter can be and a struct's field can't, and no
`DetailedHTMLProps`: ExternalLink's Rust took `AnchorHTMLAttributes` alone,
a TypeScript caller's `ref` an error, narrower than the original.
next/link's `LinkProps` held a `RefObject` alone, where TypeScript's
`Ref<T>` is a callback too.

## Decision

**`react::Ref<T>` is @types/react's `Ref<T>`, a ref a prop holds: a
`RefObject` or a callback, each the value itself in JS, an untagged enum
(ADR 0214), given as either, `.into()`. What JSX's `ref` and
`use_imperative_handle` take is `IntoRef<H, M>`: a `RefObject`, a
callback, a `ForwardedRef` or a `Ref`.**

```rust
pub struct ExternalLinkProps<'a, C: ReactNode> {
    pub href: Option<&'a str>,
    pub target: Option<&'a str>,
    pub children: Option<C>,
    #[cfg_attr(rust_js, rust_js::flatten)]
    pub props: DetailedHTMLProps<AnchorHTMLAttributes<'a>, HTMLAnchorElement>,
}
```

- **`react::DetailedHTMLProps<E, T>` is @types/react's**, what an element
  takes: its `ref`, a `Ref<&T>`, and its attributes `E`, flattened. A
  component's props flatten it as they do `E` alone (ADR 0204): one rest,
  `{href, target, children, ...props}`, and its `.d.ts` says
  `Omit<DetailedHTMLProps<AnchorHTMLAttributes<HTMLAnchorElement>, HTMLAnchorElement>, ..>`,
  so a TypeScript caller's ref of its `<a>` is checked, and one of
  another element refused.
- **`react::SVGProps<T>` is @types/react's too**, an SVG element's props,
  `SVGAttributes<T>` and its `ref`: react.dev's Logo takes it whole,
  `props: SVGProps<'_, SVGSVGElement>`, and spreads it on to its `<svg>`.
- **Given from Rust**, `ref={Some(anchor.into())}`, or a callback's
  `.into()`: `<ExternalLink ref={anchor} href="/a">` in JS, as written.
- **next/link's `ref` is a `Ref`**: a callback too, `Some(anchor.into())`.
- **The trait is `IntoRef`**, Rust's name for what converts, so `Ref` is
  TypeScript's name for the type: one name, one meaning. A component's
  own parameter that passes a ref on, `impl IntoRef<&HTMLDivElement, M>`,
  is as before but for its name.

## Consequences

- A callback a `Ref` holds returns nothing; one that returns React 19's
  cleanup is given to an element's `ref` directly, where `IntoRef` takes
  it.
- A Rust caller of a component that flattens `DetailedHTMLProps` names
  the struct, `props={DetailedHTMLProps { r#ref, attributes }}`, as a
  flattened field is named (ADR 0204); TypeScript's callers name its
  attributes.
