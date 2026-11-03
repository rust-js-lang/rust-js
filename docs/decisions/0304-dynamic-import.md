# 0304. `js::import!` of an item loads its module when it's asked for

Status: Accepted. Amends [0028](0028-js-module-imports.md), which left
dynamic `import()` for later. Amended (2026-11-02): a specifier known as
the program runs is `js::import_specifier(specifier)` (rd607). Amended
(2026-11-04): a getter's is its read off the module (#581).

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

## Amendment: a specifier known as the program runs (rd607)

React's Flight client for ESM, and for an unbundled Node server, loads the
module a server named: `import(metadata.specifier)`, of a string it was
sent. `js::import!` and `js::import_module!` take an item the crate knows,
whose module's path rust-js writes; none took a string.

**`js::import_specifier(specifier: &str) -> Promise<&'static Unknown>` is
`import(specifier)`**, JS's own, a promise of the module's namespace, any
value, as TypeScript's `import()` of a string is `Promise<any>`; rejected
where the module can't be loaded, as JS's is.

- **A function, not an arm of `js::import!`**: the macro's string arm is a
  static import for what a module does, `js::import!("./App.css")`, and
  its path arm names an item; a variable's name is a path, so an
  expression arm would read `js::import!(specifier)` as an item. Each form
  says what it's given: an item, a module by its item, a specifier.
- **No wrapper**: the call is `import(x)` itself, so a bundler sees the
  dynamic import it leaves for the host, as upstream's
  `// $FlowFixMe[unsupported-syntax]` one is.
- Case N ([0262](0262-when-rust-and-js-disagree.md)): JS's own `import()`.

Held by `test/js-functions.test.ts` ("a module is imported by a
specifier known as the program runs"): `return import(specifier);` and
`await import(base + file)`, run, a module's export read and a missing
one rejected. A binding's signature has no mutation: the test doesn't
compile without it.

## Amendment: a getter's is its read off the module (#581)

`js::import!(version)` of a getter binding, `get ./dialog.js#version`,
panicked rustc: what it gave, `m => m.version`, was the export's value,
and a call of it, `v()`, was lowered as the getter's own read, through a
static import nothing made. Of the module's namespace, `get
./dialog.js#*`, the read was a static `import * as`, so the module loaded
with this one, and `{ "*": d }` read a property no module has.

**`js::import!` of a getter is a promise of its read off the module,
`import("./dialog.js").then((m) => () => m.version)`, as the getter is as
a value, `() => version`; a call of what it gives calls that function.
Of the namespace, it's the module itself, `() => m`.**

- **Its export's value isn't the getter**: each call of a getter reads
  the export again, as an ES module's live binding changes, `export let
  version` after `version += 1`; a value read once wouldn't (case C,
  [0262](0262-when-rust-and-js-disagree.md): results change).
- **One form for a function a variable holds**: a getter held, `let v =
  version`, is `const v = () => version`, as a function binding held is
  `const t = (x) => twice(x)`, and a call of either is `v()`.
- A path past the export, `get ./x.js#a.b`, is read past it, `() =>
  m.a.b`.

Held by `test/bindings.test.ts` ("js::import! of a getter reads its export
when it's called"): a live export read before and after the module bumps
it, `[1, 2]`, the namespace the module itself, and no static import.
Mutations `held-getter-read-as-import`, `imported-getter-read-once` and
`imported-namespace-a-property`.
