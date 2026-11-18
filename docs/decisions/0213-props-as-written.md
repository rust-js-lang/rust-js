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

- **A JS class owns its JSX companion even when it implements `Default`.**
  A class and a props structure otherwise generated two macros with its
  name. The class keeps its own JSX behavior, and `Default` remains an
  ordinary Rust implementation. The class test covers construction from
  JavaScript and through Rust's `Default`. This follows ADR 0262's first
  case because the class's runtime behavior is unchanged.

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
- **Only a struct of a crate rust-js compiled is built by its companion**
  (2026-11-14, Codex's constraints on #1794): 4f7dfc12b took every props
  type the module didn't define for one with a companion, so a component
  of std's `Range<u32>`, `use std::ops::Range;`, asked for `Range!`, which
  std, compiled without rust-js, doesn't have, where it was a struct
  literal. The module's `Scope` (`props.rs`) tells, before rustc resolves
  a name, which of its props types have one: a path into this crate, its
  `crate::`, `self::`, `super::` or a module it declares, or into a crate
  its build is given with `--extern`, each compiled by rust-js; std, core
  and alloc, of the sysroot, aren't given. A bare name is of the `use`
  that names it, its last name or its `as`; one no `use` names, a glob's
  or the prelude's, is built as a literal, as before. The companion is
  called by the name the module knows it by: a `macro_rules!` of that
  name, of the module or of its parent, isn't taken for it, as the
  component's `macro` resolves where it's defined, past what's textual;
  a path-scoped macro of that name beside the `use` is rustc's E0252. A
  case of each, React's `AnchorHTMLAttributes` by a `use`, an `as` and its
  path among them, is in `test/jsx-imported-props.test.ts`.
  What's left: a library's props type a glob imports, `use ui::*;`, or
  the prelude names, is built as a literal, as before, so its flattened
  attributes fail as #1794's did. Resolving it needs rustc's names, after
  expansion, which this pass runs before; Codex's prototype resolves it
  there, and whether to take it is a decision of its own.
- **rustc's names are the truth, `Scope` their prediction** (2026-11-15,
  the lead's decision; Codex's prototype, rust-js-lang/vgpu-in-rust@143d724,
  `docs/proposals/hir-jsx-props-resolution.patch`, whose resolution step and
  marker this takes): a library's props type a glob or a kit's prelude
  imports, `use ui::prelude::*;`, is built by its companion. Each companion
  rust-js writes is marked `#[rust_js::jsx_props]`; the syntax pass predicts
  each component's props of its `Scope`, as before, and records each
  function component it predicts a literal, by where its name is written
  (`jsx_syntax/resolved.rs`). After expansion, rustc's names verify each:
  its signature's props type, an alias's target, by reference or not, has
  a companion where its module has a macro of its name that carries the
  marker (Codex's `owns_companion`), of this crate or another's. A
  prediction they prove wrong stops the compile there, before a literal is
  checked, and the crate is compiled once more with that component's
  answer (`run_resolving_props`, `main.rs`), rust-js's, `--rustc`'s and
  Clippy's alike, and the WASI build's, of the same entry point. A replay
  whose names differ again is an error, its sources having changed.
  - **A replay is rare**: a `use` that names the props, a rename, a path,
    a re-export and an alias are predicted right, the crate compiled once;
    only a glob's or a prelude's library props replay, once.
  - **What a stopped compile says is dropped**: each compile's diagnostics
    are held from its start, by an emitter rust-js builds as rustc's
    session does (`resolved::held`), until the verification says whether
    it goes on: then they're said, in order, or dropped with the compile,
    so an early lint's warning, which both compiles meet, is said once.
    One whose errors come before it could ask says them as it ends.
  - **Not verified**: a constant's or a `thread_local!`'s component, whose
    props `Scope` alone predicts, as before.
  - Case ([0262](0262-when-rust-and-js-disagree.md)): none; it compiled
    what it refused.
  - Held by `test/jsx-props-resolved.test.ts`: a glob's and a prelude's
    props built by their companion by one replay (`--props-replayed`, its
    hook), a named `use`'s and the module's own compiled once, and a
    replayed compile's early lint said once. Mutations of
    `jsx_syntax/resolved.rs`, companion-unmarked, glob-props-unpredicted
    and props-never-replayed; component-props-literal and
    imported-props-a-literal now held by a crate of its own props, and of
    a named `use`'s, compiled once, as the replay would otherwise build
    them right at a second compile's cost. No mutation of the early
    return where nothing was predicted, which saves the walk alone, nor
    of the error a replay's changed names give, which needs sources that
    change between its compiles.
