# 0347. The react crate's types are measured against @types/react, and only grow

Status: Accepted. Extends [0346](0346-next-coverage.md)'s ratchet to the
react crate; its values stay [0043](0043-react-versions.md)'s.

Case: N ([0262](0262-when-rust-and-js-disagree.md)).

## Context

The react crate follows `@types/react` (ADR 0208), and every runtime export
of the latest React is bound or left out on purpose, which
`test/react-versions.test.ts` holds. But React's API is its types too: the
props, events, refs and options its functions take and give,
`SuspenseProps`, `PreloadOptions`, `RootOptions`. Nothing said how many of
those the crate has, nor caught one lost.

## Decision

**Each export @types/react and @types/react-dom declare, a value or a type,
is listed `+` where the crate binds it and `-` where it doesn't, and the
count only grows.**

- **The measure**: `react/coverage.ts` reads each entry point a program
  imports, `react` (its namespace `React`, `React.JSX`'s as `JSX.Element`),
  `react-dom`, `react-dom/client`, `react-dom/server` and
  `react-dom/static`, and lists each export, internals (`__…`,
  `…DO_NOT_USE…`) aside, under `# react 162 of 267`.
- **What's bound** is read from rustdoc's JSON of the crate for the latest
  React, as `test/react-versions.test.ts` reads it, which now shares the
  reader: a value by its `link_name` or `test`, `<react#Suspense>` and
  `react#Children.map` of `Suspense` and `Children`; a type by its
  `types` link, `"react#MouseEvent<T>"`, or an item of its name.
- **The ratchet**: `react/coverage.txt` is that list, which
  `test/react-coverage.test.ts` holds, as `next/coverage.txt` is held.
  `next/coverage.ts` and its list moved beside their crate too, as
  builtins' and webapi's are.

## Why

- **The same promise as Next.js's** (ADR 0346): what's `-` is what's left,
  from @types/react's own list.
- **One reader of the crate's bindings**: the versions test and the
  coverage read the same links the same way.

## Consequences

- The values `-` are exactly the versions test's left-out ones: `Component`,
  `PureComponent`, `createElement`, `unstable_batchedUpdates`,
  `useFormState`, and the server and static entries' `version`.
- Today: 196 of 332, `react` 162 of 267, `react-dom` 15 of 27,
  `react-dom/client` 5 of 8, `react-dom/server` 8 of 17, `react-dom/static`
  6 of 13. The rest are types: component types (`FC`, `ComponentProps`),
  attribute unions (`AriaRole`, `HTMLInputTypeAttribute`), what
  `HTMLAttributes` copies in (`AriaAttributes`, `DOMAttributes`), and
  react-dom's options (`PreloadOptions`, `RootOptions`).

## Amendment: what isn't bound by design

What isn't bound by design is `x`, counted apart, each list with why: the
class-component API, as Rust's components are functions (the user's
choice, 2026-10-10; an error boundary is next/error's `catchError` or a JS
component's binding); and TypeScript's own, types of types that compute a
component's props, refs or elements, a string whose values are only
suggestions, and `createElement`, which `jsx!` is.

## Amendment: react-dom by @types's names

react-dom is bound whole, 25 of 25, its types under @types/react-dom's
names: `PreloadAs`, `PreinitAs`, `BrowserUsable` (were `As`, `Init`,
`Browser`); `PreloadModuleOptions` and `PreinitModuleOptions` apart (were
one `ModuleOptions`), with `PreloadModuleAs` a fetch's `RequestDestination`
and `PreinitModuleAs`; `PreconnectOptions`. A function whose options
@types makes optional has a `_with` twin, `preconnect_with`, as
`create_root_with` is.

- `FormStatus` is @types's union, `FormStatusPending | FormStatusNotPending`,
  an enum tagged by `pending: true | false` (ADR 0284): a pending form's
  `data`, `method` and `action` are there only while it's pending, where a
  struct of getters read `null` as a `FormData` and a `String`.
- A discriminated union's variant counts as TypeScript names its member,
  `FormStatusPending` of `FormStatus::Pending`.
- "old names" are `x` too, as test/react-versions.test.ts leaves them out:
  `useFormState`, `unstable_batchedUpdates`, and each entry's `version`,
  react-dom's.
