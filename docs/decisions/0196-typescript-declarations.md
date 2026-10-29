# 0196. A crate may have a `.d.ts` beside each module's JS

Status: Accepted. Amended by [0412](0412-rust-js-build-is-its-own-process.md):
every `rust-js build` writes them; the `declarations` key is gone. Extends [0019](0019-one-js-file-per-module.md) and [0117](0117-output-hooks.md);
its `.d.ts` is printed by TypeScript since [0207](0207-declarations-printed-by-typescript.md),
and imports another module's types since [0210](0210-declarations-import-module-types.md).
Amended by [0811](0811-console-create-task-may-be-absent.md): `#[rust_js::types]`
is a TypeScript type, read into the model, one it can't read an error.

Case: N ([0262](0262-when-rust-and-js-disagree.md)).

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
  text?: string | undefined;
  className?: string | undefined;
}

export function Tag(props: TagProps): ReactNode;

export default Tag;
```

| Rust | TypeScript |
|---|---|
| `bool`, a number, a 64-bit one, `&str`, `String`, `char` | `boolean`, `number`, `bigint`, `string` |
| `Option<T>`, a field of one | `T \| null \| undefined`, an optional field `?: T \| undefined` (ADR 0413) |
| `Vec<T>`, a slice, an array, a tuple | `T[]`, `[A, B]` |
| a struct with named fields, a unit-only enum | an `interface`, the union of its names |
| an enum of no variants, chrono's `enum Void {}` | `never`, a type of no values |
| a type alias, `pub type Toc = Vec<TocItem>` | `export type Toc = TocItem[]` |
| react's `Element`, `Memo<P>`, `Context<T>`, `Style` | `ReactNode`, `NamedExoticComponent<P>`, `Context<T>`, `CSSProperties` |
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
  Its `Style` is `react#CSSProperties`, as a tag's `style` is. (Amended: it
  was `any`.)
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
  react.dev's `ButtonLinkProps` TypeScript's error.) An `impl Trait`
  parameter of a trait that says no type, `value: &impl js::Defined`, is
  `unknown`, a value of any type, as a person declares one. (Amended: it
  was rustc's name for it, `impl js::Defined + 'a`, which isn't TypeScript.)
  **A type written as an alias that says what it is to TypeScript**,
  `#[rust_js::types = "react#MouseEventHandler<T>"]`, is declared by it, as
  written, a field's, an `Option` field's, or a parameter's: `onClick?:
  MouseEventHandler<HTMLButtonElement>`, an argument left to its default
  left out, `MouseEventHandler`. rustc's type is the alias expanded, so the
  field's type is read as written, its arguments lowered by
  `rustc_hir_analysis`. (Amended: it was the expansion, `(event:
  MouseEvent<HTMLButtonElement>) => void`.) **Every alias the Rust writes
  is named so, the crate's own too, wherever it's written**: a `const`'s
  type, a field's, a parameter's, a result's, an alias's, and inside
  another type, `Context<Toc>`, `Pair<Toc>`, `Vec<Toc>` a `Toc[]`; another
  module's is imported from it (ADR 0210), and a private one, which isn't
  declared, is what it stands for. One function, `as_written`, reads each
  written type's aliases, and `ts` writes each by its name where it meets
  its expansion. (Amended: an alias of the crate's was what it stands
  for, `Context<TocItem[]>` where react.dev's `TocContext` has
  `Context<Toc>`, and one of a binding's only at a type's top.)
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
- **A field of one is optional, `?: T | undefined`, as TypeScript's own data has it**:
  TypeScript reads it too, and gives what it reads to a prop whose default,
  JS's `= []`, takes `undefined` and not `null`. One marked
  `#[rust_js::nullable]` takes `null` too, `?: T | null`, where the data has
  it: react.dev's errors page gives its `ErrorDecoderContext` `{ errorMessage:
  string | null }`. Its `None` is `null` when it's made (ADR 0275).
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
- **An `impl Fn(&Item, usize) -> R` parameter is its function**, `(item:
  Item, value: number) => R`, as TypeScript declares a callback, react.dev's
  toCommaSeparatedList's `renderCallback`. (Amended: it was `unknown`, as
  any `impl Trait` parameter still is.)

## Amended by

- [0236](0236-jsx-element.md): What JSX makes is `JSX::Element`, TypeScript's `JSX.Element`

## Amendment: an `impl Fn` parameter is of its own item's parameters

React's scheduler, ported, declares `unstable_next<T>(eventHandler: impl
FnOnce() -> T)`, then `unstable_wrapCallback<A, T>(callback: impl Fn(A)
-> T + 'static)`. Its declaration read `callback: (a: A) => () => T`: an
item's erased parameters, by index, were worked out while the item before
it's were still the ones read, and `wrap`'s `T` is its parameter 1, which
`unstable_next`'s `impl FnOnce() -> T` was, `() => T`. A type TypeScript
checks a caller against was wrong.

**An item's erased parameters are worked out of its own alone**
(`Declarations::erased`): `(a: A) => T`. Case N: nothing runs a
declaration. Held by `test/declarations-impl-fn.test.ts` and the scheduler
port's snapshot; mutation `declaration-erased-of-the-last-item`.

## Amendment: a declaration carries the `//` comments before its item

React's reconciler documents its types in Flow, so its port's types carry
those comments as `//`: ReactFiberSuspenseComponent's `SuspenseState` and
each of its fields, OffscreenProps' TODOs, ReactInternalTypes' fields, and
ReactFiberLane's `// TODO: Ideally these types would be opaque ..` before
`pub type Lanes`. Only a `///` doc comment reached the `.d.ts`, as JSDoc
(ADR 0444), so its types read bare. **A declaration carries the `//` and
`/* */` comments written right before its item, past its attributes and
doc comments, as written, then its JSDoc**: a type's, a field's, a
variant's and a function's, a blank line written after them kept
(`declarations/comments.rs`, `documented`, the one reader of what's
written before an item). Not a comment after code on its line,
`retry_lane: Cell<Lane>, // ..`, which is that code's; nor one with
nothing but comments before it, the file's, a license's, which the
`.d.ts` starts with already (ADR 0383); nor one a macro made. Case N:
nothing runs a declaration. Held by `test/declarations-comments.test.ts`
and the mutations in `scripts/mutations/lower/declarations/comments.ts`.

## Amendment: a struct's bound methods are its interface's (rd144)

React's SyntheticEvent, ported, binds its prototype's functions as
methods of the event's struct, `#[rust_js::link_name = "isPersistent"]
pub fn is_persistent(&self) -> bool { unreachable!() }`, called
`event.isPersistent()`. Its interface listed only its fields, so its
declaration lost `isPersistent(): boolean`, which upstream's
BaseSyntheticEvent type declares: a type narrower than upstream's.

**A struct's interface declares each method its impls bind of the JS
object it is**, `pub`, of `self`, called `this.name(..)`
(`bindings::bound_methods`): by its JS name, of its parameters after
`self`, with its JSDoc, after its fields, `preventDefault(): void;
isPersistent(): boolean;`. Not a private one, nor one of another form, a
getter's `get name`, nor one named as a field, which the field's
property declares. Case N: nothing runs a declaration.

Held by `test/declarations-bound-methods.test.ts` and the mutations
`bound-methods-undeclared` and `bound-methods-private-declared`.

