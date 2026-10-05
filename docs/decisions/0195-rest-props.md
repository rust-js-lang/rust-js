# 0195. The props a component's struct doesn't name are `...rest`

Status: Accepted. Extends [0041](0041-react.md) and [0192](0192-next.md).

## Context

A component's props are a struct of the fields it names (ADR 0041), but
JS components take what they don't name too, and pass it on, as
react.dev's `ExternalLink` does its callers' `aria-label`:

```js
export function ExternalLink({ href, target, children, ...props }) {
  return <a href={href} target={target ?? '_blank'} rel="noopener" {...props}>{children}</a>;
}
```

A struct can't name what a JS caller may give, so a port named each
caller's, and one passing on its own, `MDX/Link`'s `{...props}`, couldn't.

## Decision

**A field of react's `Rest`, in props a component takes apart, is JS's
`...rest`: what's left of them, which `{...rest}` spreads onto an element
or another component.**

```rust
pub struct LinkProps<C> { pub href: &'static str, pub target: Option<&'static str>, pub children: C, pub rest: Rest }

pub fn ExternalLink<C: Node>(LinkProps { href, target, children, rest }: LinkProps<C>) -> Element {
    jsx! { <a href={href} target={target.unwrap_or("_blank")} rel="noopener" {...rest}>{children}</a> }
}
```

```js
export function ExternalLink({ href, target, children, ...rest }) {
```

- **Its default is `undefined`**, none: a Rust caller's
  `{..Default::default()}` writes no `rest` prop, and a spread of it
  spreads nothing.
- **One a component passes on is spread**, `<ExternalLink href="/social"
  {...rest}>`, as a JS one would.
- **Read as a field, `p.rest`, or taken apart anywhere but where the props
  are given, `let P { href, rest } = p;`, it's an error**, which says where
  to take them apart: JS's props have no `rest`.
- **Props holding what has a destructor are taken apart where they're given
  too**, `{ label, ...rest }`, each part bound the function's to drop, as a
  variable is (ADR 0098), and each with a destructor must be bound: found as
  react.dev's `ExternalLink`, whose children are a type parameter's, built by
  Cargo.

## Why

- **It's the JS a person writes**, both ways, and it's exact: a JSX test
  renders a JS caller's extra props on the element, a Rust caller's
  default with none, and a component passing its own on.

## Costs

- **What's in a `Rest` is opaque to Rust**: it's spread, not read.
