# 0210. A module's declarations import another's types

Status: Accepted. Amends [0196](0196-typescript-declarations.md).

## Context

A type of another of the crate's modules was `any` in a module's `.d.ts`
(ADR 0196): its declarations weren't imported. react.dev's `Tag` takes a
`RouteTag` and its `Breadcrumbs` a `RouteItem[]`, both `getRouteMeta`'s,
which a TypeScript caller was held to nothing of.

## Decision

**A public struct or enum of another module that has a file is imported
as a type from that file, by the specifier its JS would import it by,
and named.**

```ts
import type { RouteItem } from "./Layout/getRouteMeta.js";

export interface BreadcrumbsProps {
    breadcrumbs: RouteItem[];
}
```

- **Its declarations are its own module's `.d.ts`**, which TypeScript
  finds for the JS file's specifier.
- **A module of types only has no file**, so no `.d.ts`: its type is still
  `any`.

## Why

- **It's what its module declares**, so a caller is held to it, as Rust
  holds one.
- **It's tested**: the declarations test's component takes another
  module's struct and enum, its `.d.ts` imports them, and TypeScript finds
  a caller's wrong enum value; a module of types only stays `any`.

## Costs

- **A module of types only declares nothing**, as it has no file.
