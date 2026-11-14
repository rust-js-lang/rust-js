# 0213. A component's props are given as one flat list

Status: Accepted. Builds on [0203](0203-component-props-as-written.md),
[0204](0204-flattened-props.md), [0205](0205-flattened-chains.md) and
[0212](0212-props-defaults.md).

> **Note:** since [ADR 0438](0438-an-optional-prop-is-given-some.md), a
> left-out optional prop is `None`, so the `{..Default::default()}` in this
> decision's examples is no longer needed; it's written only to give a base
> of one's own. The current form is in the guide's
> [Optional props](../guide/react/components.md#optional-props).

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
- **Every struct a component may take has a companion**: a macro beside
  each struct whose fields are as visible as it, `#[doc(hidden)]`, which
  compiling the js, webapi and react crates' metadata pays 2.0% for (1.71 s
  to 1.75 s, the median of nine) and next's 10 ms of 0.13 s (#1794).

## Since

- **Children of a type parameter left out are React's empty node**
  (2026-10-10), `Element`'s default, `undefined`: `<Head />` of
  next/document's `HeadProps<C>`, whose `C` a `Default::default()` left
  rustc nothing to infer of. A JSX test renders a component of
  `Default` props and one of required props, each with its children
  left out, and given.
- **A companion `macro` is allowed `missing_docs`** (2026-11-09, #1098):
  as public as its struct or component, it was an undocumented item of
  the crate's, so any crate that denies the lint, `#![deny(missing_docs)]`,
  and has a `Default` struct or a component failed to build under rust-js
  alone: "missing documentation for a macro", smallvec's, base64's and
  regex's own tests among them, found by the small Crater (#1087). It's
  JSX's, not the crate's API, so rust-js allows the lint of it, as ADR
  0439 allows a component's name. Case N: nothing a program does changes.
  `test/jsx.test.ts`'s "a companion macro needs no docs in a crate that
  denies missing_docs" holds it, and that an item of the crate's own is
  still flagged.
- **A component's props of another module or crate are built by their
  struct's companion** (2026-11-14, #1794, P1): an ordinary wrapper of a
  library's component, `DeferredImage<S>(props: ImageProps<'_, S>)` of
  Next's own `ImageProps`, imported, was given its props as a struct
  literal, the companion being chosen only of the module's own props, so
  its flattened `img`'s attributes given flat, `fetchPriority`, were
  rustc's E0560 and an `Option` left out its E0063. A companion is beside
  its struct and as visible, so the `use` that imports, renames or
  re-exports the struct brings it: a component whose props type the
  module doesn't define, by a path, a `use`, a `use .. as` or a `pub use`,
  builds them by it. So each struct has one that a component of another
  module may take, every struct whose fields are each as visible as it
  (`props::props_companion`), and a type alias's forwards to its target's
  (`alias_companion`). A struct a field of which is less visible than it
  has one that refuses, saying to make them as visible: a `macro` fills
  what's left out where it's defined, so a companion of it would build
  what rustc's privacy forbids where it's used. A module's macro of a
  struct's name, an exported one at the crate's root where a companion
  would be defined twice, keeps it, and the struct has none. Each is
  `#[doc(hidden)]`. A type parameter, `P` of `fn Wrap<P>(props: P)`, and
  `Self` are no type's. A constant's component's props and a
  `thread_local!`'s memo's, which were taken to have a companion whatever
  they were, are built by the same rule, one for every component. No catalog of Next's: the companion is the
  struct's own. Case: none; it compiled what it refused.
  `test/jsx-imported-props.test.ts` holds a wrapper of another crate's
  component given its flattened attributes, each way of naming another
  module's props, a required prop left out, the refusal and a macro's
  name; `test/next.test.ts` compiles the issue's program in its app.
