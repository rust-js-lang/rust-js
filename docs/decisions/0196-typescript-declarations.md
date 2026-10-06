# 0196. A crate may have a `.d.ts` beside each module's JS

Status: Accepted. Extends [0019](0019-one-js-file-per-module.md) and [0117](0117-output-hooks.md);
its `.d.ts` is printed by TypeScript since [0207](0207-declarations-printed-by-typescript.md),
and imports another module's types since [0210](0210-declarations-import-module-types.md).

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
| `Option<T>`, a field of one | `T \| null \| undefined`, an optional field `?: T` |
| `Vec<T>`, a slice, an array, a tuple | `T[]`, `[A, B]` |
| a struct with named fields, a unit-only enum | an `interface`, the union of its names |
| a type alias, `pub type Toc = Vec<TocItem>` | `export type Toc = TocItem[]` |
| react's `Element`, `Memo<P>`, `Context<T>` | `ReactNode`, `NamedExoticComponent<P>`, `Context<T>` |
| a `Rest` of props (ADR 0195) | `[prop: string]: unknown` |
| react's events, `event::Mouse<T>` | @types/react's, `MouseEvent<T>`, `SyntheticEvent<T>` of `Event<T>` |
| `webapi`'s interfaces, `HtmlButtonElement` | the DOM lib's, `HTMLButtonElement` |
| a `dyn Fn`, an `fn` pointer | a function of what it takes and gives, `(event: MouseEvent<Element>) => void` |
| a closure's own type, a `dyn` of another trait | `(...args: any[]) => any` |
| a type of another module's, a JS object's, the rest | `any` |

- **A binding's type says what it is to TypeScript**, `#[rust_js::types =
  "react#NamedExoticComponent"]` of react's `Memo<P>`: `NamedExoticComponent<P>`,
  imported from `react`, or a global's, `"HTMLElement"`, without a module.
  Arguments written are all of its own, `<>` none: react's `Element<T>` is
  `react#ReactNode<>`, a `ReactNode` whatever its tag's element (ADR 0224).
  One written `{ [key: string]: T }` is an object of `T`s by name, its `T`
  the Rust type's, as `js::Dict<T>`'s is: `Record<string, T>` couldn't be in
  a recursive alias. **A type parameter bound by a trait that says its type
  is that type**: react's `Node` is `react#ReactNode<>`, so of
  `struct ButtonProps<C: Node> { children: C }` and `fn Button<C: Node>`,
  it's `ButtonProps { children: ReactNode }` and `Button(props: ButtonProps)`,
  as a person writes them. Rust's parameter is so a node isn't boxed, which
  TypeScript has no need of. A struct's own parameter is one where the
  struct says its bound. (Amended: it was `<C>`, which props extending
  React's attributes, whose `children` is a `ReactNode`, couldn't be,
  react.dev's `ButtonLinkProps` TypeScript's error.)
  **Another crate's untagged enum is declared in the
  module that names it**, not exported, the union of its payloads, as this
  crate's are (ADR 0225).
- **A function is typed as Rust types it**: `Box<dyn Fn(&event::Mouse)>`
  is `(event: MouseEvent<Element>) => void`, an `fn(u32, u32) -> String`
  `(value: number, value2: number) => string`, each parameter named by its
  type's last word, as a person names one, and react's events and
  `webapi`'s interfaces are @types/react's and the DOM lib's, by
  `#[rust_js::types]`, which `webapi`'s generator writes of each. A handler
  of a button's event, `event::Mouse<webapi::HtmlButtonElement>`, is
  `MouseEvent<HTMLButtonElement>`, as react.dev's Button's TypeScript has
  it. (Amended: each was `(...args: any[]) => any`, which took any function.)
- **An `Option` takes JS's `null` too**, `T | null | undefined`, as rust-js
  reads `None` `!= null` (ADR 0030): a TypeScript caller's `null`, react.dev's
  Page giving its `LanguagesContext` `Languages | null`, is `None`.
- **A field of one is optional, `?: T`, as TypeScript's own data has it**:
  TypeScript reads it too, and gives what it reads to a prop whose default,
  JS's `= []`, takes `undefined` and not `null`. One marked
  `#[rust_js::nullable]` takes `null` too, `?: T | null`, where the data has
  it: react.dev's errors page gives its `ErrorDecoderContext` `{ errorMessage:
  string | null }`.
- **A function only `js::export_default!` exports is declared, not exported
  by its name**, `declare function Recap(props: RecapProps): ReactNode;`
  before `export default Recap;`, as its JS has it. (Amended: it wasn't
  declared, so TypeScript found no `Recap`.)
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

- **`InputEvent` and `ToggleEvent` are @types/react 19's**: a project on 18's
  types finds no such type in a `.d.ts` that names one.
- **Another module's types are `any`**, until a declaration imports its own.
- **A closure's own type is untyped**, `(...args: any[]) => any`, as no
  signature names it. (Amended: every function was.)
