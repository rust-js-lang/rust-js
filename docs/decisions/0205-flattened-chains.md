# 0205. Flattened structs chain, and the props' own name is theirs

Status: Accepted. Amends [0204](0204-flattened-props.md).

## Context

TypeScript's element props are a chain, `AnchorHTMLAttributes extends
HTMLAttributes extends AriaAttributes, DOMAttributes`, and a component's
props narrow them: react.dev's `ButtonLink` is `AnchorHTMLAttributes &
ButtonLinkProps`, its own `href`, `className`, `target` and `type` taken
apart, and `...props` the rest, which hold none of them. ADR 0204 refused
both: a flattened struct with one of its own, and a name both have.

## Decision

**A flattened struct may flatten another, and a name the props have
that a flattened struct has too is the props': what the rest holds
leaves it out, as TypeScript's `Omit` does.**

```rust
pub struct Anchor { pub href: Option<&'a str>, #[rust_js::flatten] pub html: Html }
pub struct ButtonLinkProps { pub href: &'a str, #[rust_js::flatten] pub props: Anchor }
```

```ts
export interface Anchor extends Html { href?: string; }
export interface ButtonLinkProps extends Omit<Anchor, "href"> { href: string; }
```

- **A flattened field is read through**, `props.html.title` is
  `props.title`; read whole, it's still an error.
- **In JSX, each level made there is its fields**, `<ButtonLink href="/a"
  target="_blank" id="i">`, but a name the props have, which a Rust caller
  giving it is an error: theirs is the one JS has.
- **Taken apart, a field the pattern leaves is named still**, `{ href,
  className: _className, ...props }`, so the rest holds only what isn't
  the props' own: `Rest`'s too.
- **A binding's type of TypeScript's may be generic**, `rust_js::types =
  "react#AnchorHTMLAttributes<HTMLAnchorElement>"`: its name is imported.

## Why

- **It's TypeScript's props**: an element's attributes extend a chain,
  and a component that names some has the rest without them, the
  `{ href, ...props }` a JS component takes apart.
- **It's exact**: the props' own value is the only one JS has, so a Rust
  caller can't give another that would be lost, and a flattened struct's
  field of that name is always `None`.
- **It's tested**: a JSX test renders a chain called by Rust and by
  `createElement`, with a shadowed name, a field read through, and one a
  pattern leaves, and refuses a shadowed name given and the chain read
  whole; the declarations test checks the `extends Omit<..>`, and a
  generic React type extended, against TypeScript.

## Costs

- **A shadowed name's own field is still in the flattened struct**, as
  Rust has it, always `None`.
