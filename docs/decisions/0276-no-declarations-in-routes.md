# 0276. A Pages Router route gets no declarations beside it

Status: Accepted. Extends [0196](0196-typescript-declarations.md) and
[0273](0273-path-modules.md).

Case: N ([0262](0262-when-rust-and-js-disagree.md)).

## Context

A crate with `declarations = true` has a `.d.ts` beside each module's JS
(ADR 0196). In Next.js's `pages/`, every file is a route: Next.js's webpack
build leaves a `.d.ts` out, but Turbopack, its default, builds
`pages/errors/[errorCode].d.ts` as a page, and fails on its `import type`,
as react.dev's errors page found.

## Decision

**A module in a directory of routes gets no `.d.ts`.** `checkCargo`'s
`routes` names them, and `rust-js-next` gives a Next.js app's `pages/` and
`src/pages/`. Its JS is written as any module's is.

## Why

- **A route is Next.js's to import**, not TypeScript's: nothing needs its
  declarations, and one there breaks the build.
- **It's tested**: the Next.js example, with declarations, builds a page of
  `pages/codes/[code].rs` with none beside it and another module with one.
