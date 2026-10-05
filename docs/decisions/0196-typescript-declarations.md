# 0196. A crate may have a `.d.ts` beside each module's JS

Status: Accepted. Extends [0019](0019-one-js-file-per-module.md) and [0117](0117-output-hooks.md).

## Context

TypeScript that imports a module rust-js wrote types it from its JS. A
component that takes its props apart, `function Tag({ text, variant,
className })`, is then one each of whose props is required, as TypeScript
reads a JS function's, where Rust's `Option` fields are optional: react.dev's
`PageHeading.tsx`, `<Tag variant={tag} />` of the `Tag` ported to Rust, didn't
type-check.

## Decision

**A crate's `declarations = true`, in `[package.metadata.rust-js]`, is a
`.d.ts` beside each module's JS that exports something, `Tag.d.ts` beside
`Tag.jsx`: what it exports, as Rust types it, each by its JS name.**

```ts
import type { ReactNode } from "react";

export type RouteTag = "foundation" | "intermediate";

export interface TagProps {
  variant: RouteTag;
  text?: string;
  className?: string;
}

export function Tag(props: TagProps): ReactNode;

export default Tag;
```

| Rust | TypeScript |
|---|---|
| `bool`, a number, a 64-bit one, `&str`, `String`, `char` | `boolean`, `number`, `bigint`, `string` |
| `Option<T>`, a field of one | `T \| undefined`, an optional field |
| `Vec<T>`, a slice, an array, a tuple | `T[]`, `[A, B]` |
| a struct with named fields, a unit-only enum | an `interface`, the union of its names |
| react's `Element`, `Memo<P>`, `Context<T>` | `ReactNode`, `NamedExoticComponent<P>`, `Context<T>` |
| a `Rest` of props (ADR 0195) | `[prop: string]: unknown` |
| a closure, a function | `(...args: any[]) => any` |
| a type of another module's, a JS object's, the rest | `any` |

- **A binding's type says what it is to TypeScript**, `#[rust_js::types =
  "react#NamedExoticComponent"]` of react's `Memo<P>`: `NamedExoticComponent<P>`,
  imported from `react`, or a global's, `"HTMLElement"`, without a module.
- **What it can't type is `any`**, so TypeScript holds a caller to no more
  than Rust does: another module's type, which would need its import, a
  binding's JS object, an enum with fields.
- **A Cargo build's is beside the Rust too**, as its JS is (ADR 0101), and
  the manifest names it, `types`, for a build tool.
- **Off unless a crate says**: a JS app's modules have no use for one.

## Why

- **TypeScript checks what's written against Rust's types**: a test runs
  `tsc` on TSX that uses a crate's components, an `Option` prop left out, a
  JS caller's own props passed on, and finds only a prop of the wrong
  variant an error; another has a Cargo build's beside its Rust.

## Costs

- **Another module's types are `any`**, until a declaration imports its own.
- **Closures are untyped**, `(...args: any[]) => any`.
