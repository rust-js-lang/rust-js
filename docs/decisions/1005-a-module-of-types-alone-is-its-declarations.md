# 1005. A module of types alone is its declarations

Status: Accepted. The user's decision. Amended: not the crate's root. Extends
[0196](0196-typescript-declarations.md), whose `.d.ts` was written beside each
module's JS.

Case ([0262](0262-when-rust-and-js-disagree.md)): none; what JS runs is as
before, and TypeScript is given the types it was missing.

## Context

React DOM's `PluginModuleType` and `TopLevelEventTypes` are Flow files of
types alone, which other modules import with `import type {AnyNativeEvent}
from './PluginModuleType'`. Ported, each is a module of `pub type`
aliases, and the user decided a module of types is its Rust and a
`.d.ts`, upstream's `.js` removed. rust-js wrote nothing of one: a module
was JS only where it had a function, a constant, a re-export, a default or
an import of a file for what it does, and its declarations only beside that
JS. So no `.d.ts`, and TypeScript importing its types found no module; a
struct of another module whose declarations weren't written was a
`[prop: string]: unknown` where it was flattened.

## Decision

**A module that exports a type, a `pub type`, struct, enum or union, is one
of the crate's modules; one of types alone, with no JS of its own, is its
`.d.ts` alone, no `.js` and no map** (`LoweredModule::types_only`, and the
manifest's `types_only`). Another module imports its types from where its
JS would be, `import type { PluginName } from "./plugin_types.js"`, which
TypeScript finds the `.d.ts` of, as it does of any module's.

- **Its manifest entry** keeps where its JS would be, `file` and `map`,
  as what its importers name, unwritten and no artifacts, and says
  `"types_only": true`; its `types` is its `.d.ts`, an artifact. A host
  writes its `.d.ts` in source beside its Rust, and no JS: `rust-js
  build`'s `writeInSource` and the files a crate's checks are given skip
  it. A consumer of a library reads its types from it as it does of any
  module's declarations.
- **Without `--declarations`** nothing is written of it, as before.
- **Not the crate's root** (amended): its entry, which a host runs and a
  package's `main` names, is its JS, a file of nothing where it has no
  JS, as before. `pub trait Marker {} impl Marker for u32 {}` wrote no
  `lib.js` at all, and `test/traits.test.ts`' "an empty trait
  implementation needs no function body" failed on main; its mutation is
  `types-only-root-unwritten`.

## Why

- **It's the user's decision**, and it's what TypeScript reads: a `.d.ts`
  is a module of types to it, and no bundler imports one.
- **One rule for which modules are files**: a module is one where it has
  anything to say, its JS or its types.

## Alternatives

- **A JS file of nothing beside it**: the `.js` upstream doesn't have, and
  the user decided against.
- **No `file` in the manifest of one of types alone**: every host and the
  consumer key a module by its `file`; where its JS would be is still where
  its types are imported from.

## Consequences

- A module of types alone in a crate built before has a `.js` beside its
  Rust that this build doesn't write: `rust-js build` removes it, by its
  ledger, where no one has changed it.
- `test/declarations.test.ts`' `CardProps`, which flattens another
  module's struct, extends it, `extends Far`, where it was `[prop: string]:
  unknown`.

## What holds it

- `test/cargo-in-source.test.ts`, "a Cargo build's declarations in source
  are beside each module's JS": `plugin_types.d.ts` and no
  `plugin_types.js`, imported from by `App.d.ts`.
- `test/declarations-libraries.test.ts`, "a library's module of types
  alone gives its types to its consumer's declarations", checked by
  TypeScript.
- `test/declarations.test.ts`: `far.d.ts`, and no `far.js`.
- Mutations `types-only-module-unlisted` (`lower/analysis.rs`),
  `types-only-module-written-as-js` (`lower/pipeline.rs`) and
  `types-only-module-needs-js` (`library.rs`).

## Amendment: one another module's JS imports keeps its JS

Case ([0262](0262-when-rust-and-js-disagree.md)): C, a crash where rustc's
`imports/use-mod.rs` ran: Rust's behavior kept.

`pub use foo::bar::{self, First};` of a module of types alone is
`export * as bar from "./foo/bar.js"`, and `foo/bar.js` wasn't written: the
program threw as it loaded, "Cannot find module". Only a declaration's
`import type` may name a module whose JS isn't there.

**A module of types alone that another module's JS imports or re-exports
keeps its JS, though nothing is in it** (`link::keep_imported`, as the
linker is what knows every module's imports): `foo/bar.js` is written
beside its `.d.ts`, and `import * as` of it is an empty namespace, as a
module of nothing but types is to JS.

Held by corpus `module_of_types_reexported`, against native Rust, rustc's
`imports/use-mod.rs`, and the mutation `imported-types-module-unwritten`.
