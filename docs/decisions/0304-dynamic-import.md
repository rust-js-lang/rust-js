# 0304. `js::import!` of an item loads its module when it's asked for

Status: Accepted. Amends [0028](0028-js-module-imports.md), which left
dynamic `import()` for later.

Case: A ([0262](0262-when-rust-and-js-disagree.md)).

## Context

react.dev loads what it needs late, so a page doesn't download it:

```ts
const {linter} = await import('@codemirror/lint');
const {runESLint} = await import('./runESLint');
const SandpackRoot = lazy(() => import('./SandpackRoot'));
```

rust-js imported every module statically. ReScript has `import(M.x)`, a
promise of the value, compiled to `import("./M.mjs").then(m => m.x)`, of a
module of its own or of an external's, and `module M = await M`, the
module itself; what's only imported so gets no static import.

## Decision

**`js::import!(item)` is a `Promise` of `item`, a pub function of the
crate's or a binding's of a JS module, whose module is loaded when it's
asked for; `js::import_module!(item)`, of a module's default export, is a
`Promise` of the module itself, a `js::Module<T>`, as React's `lazy`
takes. Nothing imports either module statically.**

```rust
let runESLint = js::import!(super::runESLint::runESLint).await;
let module = js::import_module!(root::Root).await;
```

```js
const { runESLint } = await import("./runESLint.js");
const module = await import("./root.js");
```

- `import(spec).then((m) => m.item)` is what it is, as ReScript's is; a
  `const` of its awaited value is `const { item } = await import(spec)`,
  another name `{ item: name }`.
- A crate's module is by its file's specifier, as a static import's is;
  a binding's, `./math.js#twice`, is by its link name's module.
- `js::import!("./App.css")`, of a string, is still the module's own
  import, as JS's `import "x"` statement is to its `import("x")`.

## Why

- **It's the JavaScript a person writes**: react.dev's `await import(..)`.
- **It's the same program**: what's imported is loaded where it's asked
  for, before what's given it runs.
