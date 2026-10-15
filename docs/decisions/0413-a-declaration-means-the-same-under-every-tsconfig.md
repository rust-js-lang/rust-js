# 0413. A declaration means the same under every tsconfig

Status: Accepted. Amends [0196](0196-typescript-declarations.md) and
[0212](0212-props-defaults.md).

Case: R ([0262](0262-when-rust-and-js-disagree.md)): a caller's TypeScript
is held to what the Rust takes, no more, whatever the app's settings.

## Context

A crate's `.d.ts` (ADR 0196) is read by the app's TypeScript, under the
app's `tsconfig.json`, and some of its settings change what a declaration
means. A field a caller may leave out was `?: T`: under
`exactOptionalPropertyTypes`, which strict apps turn on, that refuses
`undefined`, so `<Badge title={maybeTitle} />` of a `string | undefined`
didn't type-check, where Rust's `Option` takes it as `None` and JS reads a
missing prop and an `undefined` one alike. The declarations were only ever
checked under one setting, `strict` with `moduleResolution: "bundler"`.

## Decision

**A declaration is written one way, which means the same under every
setting an app's TypeScript may have: an optional field is `?: T |
undefined`.**

```ts
export interface BadgeProps {
    label?: string | null | undefined;
    title?: string | undefined;
    data?: unknown;
}
```

- **One that may be left out takes `undefined`**, an `Option` field's, a
  `#[rust_js::nullable]` one's and one with a default (ADR 0212), as
  @types/react writes each of its own: `onClick?: MouseEventHandler<T> |
  undefined`. One of `unknown` or `any` has it already, and is written as
  it is.
- **A type written as one of TypeScript's keywords is that keyword**: js's
  `Unknown`, `#[rust_js::types = "unknown"]`, is `unknown`, as the model's
  own `unknown` is, not a type named `unknown`.
- **Not one form per setting**: rust-js doesn't read the app's
  `tsconfig.json`. A declaration that meant one thing under one setting and
  another under another would be two forms of one construct; one that
  means the same under all is one.

## Why

- **Rust's meaning, kept**: a caller passing what the Rust takes isn't
  refused, and one passing what it doesn't take is, under any setting.
- **It's tested under each**: every type-check in
  `test/declarations.test.ts` runs under `bundler`; under the strictest,
  Vite's template's `verbatimModuleSyntax`, `isolatedModules` and
  `erasableSyntaxOnly` with `exactOptionalPropertyTypes`; and under
  `nodenext`. Each must find the same errors, `skipLibCheck` off, so the
  declarations are checked too.

## Consequences

- **Every optional field's declaration has `| undefined`**: an app's
  committed `.d.ts` changes at its next `rust-js build`, and `rust-js
  build --check` says so until it's run.
- **A setting that changes a declaration's meaning is added to the test**
  when one is found; rust-js still writes one form.
