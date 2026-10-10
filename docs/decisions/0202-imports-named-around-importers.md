# 0202. An import is named in each file that imports it

Status: Accepted. Amends [0028](0028-js-module-imports.md).

Case: N ([0262](0262-when-rust-and-js-disagree.md)).

## Context

An import's name is the same in every file that has it, unique among the
crate's imports, and was named around every module's items. react.dev's
`ButtonLink` imports next/link's default, `Link`, and its `IconLink`
module has a function of its own, `fn Link`, which ButtonLink's file
never sees: every file's import was `import Link$1 from "next/link"`.

Named around only the modules that import it, it still was, once one of
them had its own: react.dev's `MDX/Link` imports next/link beside its
`fn Link`, and `Breadcrumbs`, `ButtonLink`, `DocsFooter` and `BlogCard`
each had `import Link$1 from "next/link"`. (Amended.)

## Decision

**An import is named in each file, around the globals, that file's items
and its other imports: another module's item of its name is no reason to
rename it, and one of the file's own renames it there only.**

```js
// components/ButtonLink.jsx, beside components/MDX/Link.jsx's `function Link`
import Link from "next/link";
```

```js
// inner/leaf.js, of its own `fn join` and a `path_join` that's node:path's `join`
import { join as join$1 } from "node:path";

function join(greeting) {
```

- **A default import is named as the file's `use` renames it**: `use
  next::link::Link as NextLink` is `import NextLink from "next/link"`, as
  react.dev's MDX `Link` names it beside its own; a `static`'s is camel
  case, as an asset's own name is. (Amended.)
- **Every module names every import**, so a module's locals avoid it, as
  they did: a library's export (ADR 0100), or one a trait's default
  copied into an impl uses (ADR 0049), is used by modules known only once
  they're lowered.

## Why

- **A name is the file's**: one in a file that doesn't import it is no
  name of that file's import, nor is one another file renames. A module
  of the crate's own, imported by the importing one, is the linker's to
  alias, as it was.
- **It's tested by the `imports` snapshot**: `inner/leaf.rs` has a `fn
  join` of its own and imports `node:path`'s `join` too, `join$1` there,
  and the root's `import { join } from "node:path"` keeps its name.
  A JSX test's module renaming next/link's default has `import NextLink
  from "next/link"`, another's keeps `Link`, and a `static` renamed
  `banner_img` is `bannerImg`.

## Amendment: a crate module's named import by its `use`'s name

A `use` that renames another module's function of the crate imports it
by that name, `use inner::client as sandpack` is `import { client as
sandpack } from "./inner.js"`, as react.dev's MDXComponents imports
`{SandpackClient as Sandpack}` and keeps `Sandpack` in its table, a
default import already was. A module test imports two so; a mutation
drops the name. The JSX snapshots that rename a component, `Card as
Panel`, `MEMO as Cached` and `THEME as Theme`, print those names.
