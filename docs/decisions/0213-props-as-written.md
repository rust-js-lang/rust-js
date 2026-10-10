# 0213. A component's props are given as one flat list

Status: Accepted. Builds on [0203](0203-component-props-as-written.md),
[0204](0204-flattened-props.md), [0205](0205-flattened-chains.md) and
[0212](0212-props-defaults.md).

Case: N, A ([0262](0262-when-rust-and-js-disagree.md)).

## Context

`jsx!` gave a component a struct literal of its props, so a Rust caller
named the flattened struct and filled it, and gave every field, the rest
from a base:

```rust
<ButtonLink href="/x" props={AnchorHtmlAttributes { download: Some("f"), ..Default::default() }} {..Default::default()}>
```

where a JS caller writes `<ButtonLink href="/x" download="f">`.

## Decision

**A props struct has a companion, a macro of its name, which builds it
from the names `jsx!` gives it: each of its own fields, and every other
name its flattened field's, by that struct's companion. A field not given
is left out, of an `Option`, a `Rest` or one with a default; a required
one not given is an error that names it.**

```rust
<ButtonLink href="/x" download={Some("f")}>{"Docs"}</ButtonLink>
```

```jsx
<ButtonLink href="/x" download="f">Docs</ButtonLink>
```

```text
error: missing prop `href` of `ButtonLinkProps`
```

- **It's made for the crate's components' props structs and every
  `Default` struct**, as a flattened one is, beside the struct, of its
  visibility; a component whose props have one is given them by it.
- **What's left out is `undefined`**, `react::__omitted()`, which JSX
  doesn't write, and the component's own default then is (ADR 0212);
  children not given are their type's `Default`.
- **The flattened field itself may be given whole**, `props={props}`, a
  component's own passed on, written `{...props}`; given whole and names
  of it too is an error.
- **A component's own, updated, is its rest spread, then the update**:
  `props={AnchorHTMLAttributes { html: HTMLAttributes { class_name, ..props.html
  }, ..props }}` of a component's flattened `props` is `{...props}
  className={..}`, as react.dev's Link gives `ExternalLink` its classes.
  Its props pattern's rest, `...props`, holds no key the pattern names, so
  what it gives is spread, not each field, and one the other component
  has too, `href`, isn't given twice; a flattened field read whole as a
  base, `..props.html`, is the object its parent is. The spread is before
  the update, which holds, as Rust's does. (Amended.)
- **The companion writes the fields in the order they're declared**, the
  flattened one where it is: a literal in order needs no `const`s, so an
  `if` of next/link's `Link`, whose `className` comes after its anchor's
  props, is a conditional in its JSX, as react.dev's Link chooses its
  link. Rust makes a flattened struct declared first first. (Amended:
  it was last.)
- **A name nothing has is rustc's error**, at the name.
- **With a base, `{..base}`, it's the struct literal with it**, as before.
- **The props are as written**, by where each one's value is (ADR 0203),
  a flattened struct's too; Rust makes them in the companion's order, so
  what does something, reordered, is made first in that order, and an
  object made here, a flattened struct's, is made each of its values, so
  it stays one taken apart where it's given. Children the props declare
  before a prop that does something, which Rust makes first, are made
  first too, but where they read only variables that never change, a
  state's value or a parameter, and do nothing: those read the same
  after, so react.dev's Button keeps `<span>{expanded ? .. : ..}</span>`
  in place beside a `className` its `cn` makes. (Amended: they were made
  first always, and every prop with them.)
- **A `Default` struct of only `Option`s and the like, none flattened,
  is its literal with its `Default`**: its companion has no slot for each
  of its fields, which React's `HtmlAttributes` has hundreds of.

## Why

- **It's how JSX is written**: one list, the component's props and the
  element's alike, what isn't given left out.
- **It's Rust's**: a struct literal, checked by rustc, a required field
  said missing, a wrong name rustc's error, each value of its field's
  type.
- **It's tested**: a JSX test gives a component of a flattened chain its
  props as one list, in an order of the caller's own, with a default and a
  required prop, and checks the JSX, what renders, and the order what does
  something is made in; a component passes its own flattened props on
  whole; and it refuses a missing prop, a wrong name, and both a
  flattened struct and names of it.

## Costs

- **A Rust caller still writes `Some(..)`** of an `Option` prop.
- **A component's props type beside it, or a `Default` struct, has a
  companion**: one elsewhere is given a struct literal, as before.

## Since

- **Children of a type parameter left out are React's empty node**
  (2026-10-10), `Element`'s default, `undefined`: `<Head />` of
  next/document's `HeadProps<C>`, whose `C` a `Default::default()` left
  rustc nothing to infer of. A JSX test renders a component of
  `Default` props and one of required props, each with its children
  left out, and given.
