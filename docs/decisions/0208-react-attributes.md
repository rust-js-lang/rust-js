# 0208. Each element's attributes, as @types/react types them

Status: Accepted. Uses [0204](0204-flattened-props.md), [0205](0205-flattened-chains.md)
and [0206](0206-typescript-module.md).

## Context

A React component's props are often an element's and its own,
TypeScript's `React.AnchorHTMLAttributes<HTMLAnchorElement> &
ButtonLinkProps`, as react.dev's `ButtonLink` is. The react crate's
elements take attributes as methods (ADR 0043), one set for every
element, from React DOM's own tables; which attributes an `<a>` takes, and
of what types, only @types/react says.

## Decision

**`react::attributes` is each of @types/react's element attribute
interfaces as a struct, generated from it by `react/attributes.ts`, which
reads it with TypeScript's parser: a struct a component's props flatten.**

```rust
use react::attributes::AnchorHtmlAttributes;

pub struct ButtonLinkProps<'a> {
    pub href: &'a str,
    #[cfg_attr(rust_js, rust_js::flatten)]
    pub props: AnchorHtmlAttributes<'a>,
}
```

```ts
export interface ButtonLinkProps extends Omit<AnchorHTMLAttributes<HTMLAnchorElement>, "href"> {
    href: string;
}
```

- **`HtmlAttributes` and `SvgAttributes` are flat**, `AriaAttributes` and
  `DOMAttributes` in each, and an element's, `AnchorHtmlAttributes`, its
  own and its base flattened, as its interface extends it: `html`, or
  `media` of `AudioHtmlAttributes`.
- **Each is declared React's type**, `rust_js::types =
  "react#AnchorHTMLAttributes<HTMLAnchorElement>"`, its element the one
  `JSX.IntrinsicElements` gives it, or its base's, `HTMLElement`, of
  several.
- **Each field is optional, its type its values'**: a string, or a type
  that takes one, `&'a str`; a number `f64`; a boolean or `Booleanish`
  `bool`; an event handler `Box<dyn Fn(&event::Mouse)>`, its event as
  `elements.rs` has it; `style` a `Style`, which a tag's `style` takes as it
  is, `None` none, as react.dev's Button's `style={style}` passes its own
  on. (Amended: it took only a `Style`.) A `Style` spreads another over
  its own, `Style::new().width(w).spread(custom)`, `{ width: w,
  ...custom }`, as react.dev's console box does; an object a binding
  builds takes `prop ...` as a spread. (Amended.) A type alias is followed in the
  file. What `lib.rs` writes by hand, `children`, `ref`, `key`,
  `dangerouslySetInnerHTML`, `action` and `formAction`, isn't one.
- **`next/link`'s `LinkProps` flattens `AnchorHtmlAttributes`**, as
  Next.js types it, where it held a `Rest` (ADR 0200), its own `className`,
  `target`, `rel`, `id` and `aria-label` shadowing those.
- **`bun run generate:attributes`** writes `react/src/attributes.rs`, and
  a test checks it's what the generator makes of the pinned @types/react.

## Why

- **It's React's own typing**: TypeScript that uses a Rust component sees
  @types/react's interface, and Rust sees its fields, by their types.
- **It's read by TypeScript's parser**, not patterns (ADR 0206): every
  member taken apart, none left out.
- **It's tested**: the generated file is what @types/react makes, and a
  component that flattens `AnchorHtmlAttributes` is declared as extending
  `Omit<AnchorHTMLAttributes<HTMLAnchorElement>, ..>`, which TypeScript
  checks a caller's `download`, `aria-label` and `onClick` against, and
  renders a JS caller's.

## Costs

- **A type of a string and more is a string**: `src`'s `string | Blob`
  is given a string from Rust, and `crossOrigin`'s literals any string.
- **@types/react 19.3's**: an attribute React 18 doesn't know is still a
  field, where `elements.rs`'s methods are gated by release.
