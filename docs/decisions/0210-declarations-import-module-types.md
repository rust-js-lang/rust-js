# 0210. A module's declarations import another's types

Status: Accepted. Amends [0196](0196-typescript-declarations.md). Amended: a library crate's type is imported from its declarations too.

Case: N ([0262](0262-when-rust-and-js-disagree.md)).

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

## Amendment: a library's type

A library crate's type was `any` in its consumer's declarations: React's
reconciler reads `CacheContext`, a `ReactContext<Cache>` of the react
crate's, and its `.d.ts` said `context: any`, while the same type of its
own crate was `Context<number>`.

**A library's type that its own declarations export is imported from its
module's `.d.ts`, by the specifier its JS imports the library's
functions by, and named:**

```ts
import type { Context } from "../../a/src/context.js";

export function read(context: Context<number>): number;
```

- **The library says which**, in its contract (ADR 0100): each type its
  modules' declarations export by its name, by rustc's key for it, its
  module and its name, `types`: a public struct, enum or union of a
  module of its own that declarations declare, by the one rule a module's
  declarations import another's type by (`declarations::declares_type`),
  and whether it's its module's default, `import type Complex from ..`,
  as another module's of the crate's is. The consumer reads the answer,
  not the library's `.d.ts`.
- **Only where its declarations are written**: a library compiled
  without them has no `.d.ts` to import from, its manifest's module no
  `types`, so its type is still `any`, as a library written by an older
  compiler, whose contract says no types, has it.
- **Its file's specifier is relative to the consumer's output**, as a
  library's function's is, and a copy beside the source relinks it, as
  its JS's import.

Case N ([0262](0262-when-rust-and-js-disagree.md)): a declaration only
checks TypeScript that calls the JS; what runs is the same.

Tested by `test/declarations-libraries.test.ts`: a consumer's
declarations import a library's `Context<T>` and TypeScript finds a
caller's wrong value; a library's default class is imported as its
default; a library without declarations stays `any`. Its mutations are
`library-type-declared-any`, `library-type-without-declarations-imported`
and `library-default-type-imported-by-name`.
