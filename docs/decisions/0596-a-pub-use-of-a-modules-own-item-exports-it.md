# 0596. A `pub use` of a module's own item exports it by that name

Status: Accepted; amended (an item its own module renames is exported by
that name alone, below). Extends [0240](0240-reexports.md)'s re-exports
and [0573](0573-every-rust-name-is-camel-case-in-js.md)'s names.

Case ([0262](0262-when-rust-and-js-disagree.md)): A.

## Context

React's scheduler exports its own functions, and its priorities, by the
names its API has:

```js
export {
  ImmediatePriority as unstable_ImmediatePriority,
  ...
  shouldYieldToHost as unstable_shouldYield,
  getCurrentTime as unstable_now,
};
```

In Rust that's `pub use`:

```rust
#[cfg_attr(rust_js, rust_js::name = "unstable_now")]
pub use self::get_current_time as unstable_now;
#[cfg_attr(rust_js, rust_js::name = "unstable_ImmediatePriority")]
pub use scheduler_priorities::IMMEDIATE_PRIORITY as UNSTABLE_IMMEDIATE_PRIORITY;
```

A `pub use` of another module's function was a re-export (ADR 0240); one of
the module's own was dropped, and one of a constant too. And a `pub use ..
as` named the item in the module's own code as well: the scheduler would
have imported `ImmediatePriority` as `UNSTABLE_IMMEDIATE_PRIORITY` and read
it so.

## Decision

**A `pub use` exports its item by the name it gives**, in the JS and the
`.d.ts`, whatever module the item is of:

- another module's function or constant: `export { IMMEDIATE as
  unstable_ImmediatePriority } from "./priorities.js";`, as before for a
  function;
- the module's own function by another name: `export { currentTime as
  unstable_now };`, after its functions, as `export default` is.

The name is the `use`'s `#[rust_js::name]`, as any item's is, or its
name camelCase, `time_now` as `timeNow` (ADR 0573).

**The module reads the item by its own name**, not the name it exports it
by: a `pub use` renames what others import, never what its own code says,
as JS's `export { a as b }` doesn't. A private `use .. as` still renames
the import, `import { client as sandpack }`.

The `.d.ts` model has one kind for both, `{ kind: "export", from?, names }`,
`export { .. }` with or without `from`, as JS has one statement.

## Consequences

- An alias's local is an exported function, so nothing reclaims its name
  (ADR 0352).
- A `pub use` of a type still re-exports nothing (ADR 0240).

## What holds it

- `test/modules.test.ts`, "a pub use by another name of the module's own
  function exports it by that".
- `test/manifest.test.ts`, "a manifest says what each module exports":
  the alias is among them.
- The guide's [Emitting modules](../guide/project/modules.md#re-exports).
- Mutations: the own alias dropped, a constant's re-export dropped, a
  `pub use` renaming its own module's reads, the alias unprinted, not in
  the manifest, not in the `.d.ts`.

## Amendment: a list that names a type is text

The model's `export` has no type-only name, so reading
`export { type AllowSharedBuffer, Buffer }` as one dropped the `type`. A
list that names a type stays text, as `export type { .. }` does. The node
and next coverages read the `export` kind, a name imported into the module
as well as its own, so @types/node's buffer exports stay bound. Held by
`test/typescript.test.ts`, "read keeps an export list that names a type as
text", and the node and next coverage tests.

## Amendment: an item its own module renames is exported by that name alone

Case ([0262](0262-when-rust-and-js-disagree.md)): A. No Rust program
can tell; what JS can import of the module changes, to upstream's.

The scheduler's port, `pub use self::get_current_time as unstable_now;`
and `pub use self::should_yield_to_host as unstable_should_yield;`,
exported each by both names, `export function shouldYieldToHost()` and
`export { .., shouldYieldToHost as unstable_shouldYield }`, where
upstream declares `function shouldYieldToHost()` and exports it by
`unstable_shouldYield` alone. The `pub fn` is `pub` only because Rust
can't say "public as the alias alone": a `pub use` of a private item is
E0364.

**A module exports its own function or constant that a `pub use` of it
names by another name by that name alone**: declared without `export`,
`function shouldYieldToHost() {..}`, `const LIMIT = 9;`, `declare
function getCurrentTime(): number;` in the `.d.ts`, and listed in the
one `export { .. }` after its code, each alias in the order the `pub
use`s are written, with what else the module lists there (a `let` an
exported function became, ADR 0610's).

**Every importer imports it by that name**, which
`default_exports::exported_as` answers, as it answers a default (ADR
0773, amended): a module of the crate, another crate through the
library's manifest, whose item says it (`Item::exported_as`, which
replaces `default: bool`: `"default"` or the alias), a `pub use` of it
in another module, a module's default naming it. The importer names
it as its Rust does:

```rust
// report.rs
pub fn report() -> f64 {
    crate::scheduler::get_current_time() * 10.0
}

// lib.rs
use scheduler::unstable_now;
```

```js
// report.js
import { unstable_now as getCurrentTime } from "./scheduler.js";

// lib.js
import { unstable_now } from "./scheduler.js";
```

A path that names the item by its own name, or through a re-export
without a `use` (`scheduler::MAX`), reads it by its own name, as a
default import is named after the item (ADR 0773); a `use` of the alias
reads it by the name it's exported by, `unstable_now`, not camelCase
`unstableNow` (`default_exports::alias_named`). An import names what it
imports in order, `import { MAX as LIMIT, unstable_now as
getCurrentTime }`. Where an item has two aliases, importers import the
first.

**The scheduler's JS changes**: `function shouldYieldToHost()`, where
it was `export function`; `export { getCurrentTime as unstable_now,
shouldYieldToHost as unstable_shouldYield };`, without the trailing
`getCurrentTime` its `let` had been listed by; and its `.d.ts` `declare
function getCurrentTime(): number;` and `declare function
shouldYieldToHost(): boolean;`. Its package, `export * from
'./src/forks/Scheduler'`, has upstream's names and no others.

### Why

- **Upstream's module**: React's scheduler and `ReactChildren.js`
  declare their functions and list them once, `export {
  forEachChildren as forEach, mapChildren as map, .. }`.
- **What's exported is what's read**, as a default is exported once
  (ADR 0773): nothing outside reads the item by its own name.

### What can't be written

React's `ReactChildren.js` lists `toArray` by its own name in that list.
Rust has no spelling of it: `pub use self::to_array;` is E0255, the name
defined twice, and so is `pub use self::to_array as to_array;`. Such a
function stays `export function toArray`. Listing every function a
module exports by name in its list once it has an alias would make one
`pub fn` two forms by what else its module says, so it isn't done
without a decision of its own.

### Alternatives

- **Exported by both names, as before**: a second export of the module
  upstream doesn't have, which an `export *` passes on.
- **An importer reading the alias wherever it names the item**: a
  caller that writes `get_current_time()` reading `unstable_now()`; the
  Rust it's written from says otherwise.

### What holds it

- `test/modules.test.ts`, "a pub use by another name of the module's own
  function exports it by that" (a function and a constant, declared,
  the list in order, nothing by its own name) and "a module's own item a
  pub use renames is imported by that name" (a path, a `use` of the
  alias, a `pub use` of it in another module, run).
- `test/cross-crate-items.test.ts`, "a library's item its module exports
  by another name is imported by that": the manifest's `exported_as`, the
  consumer's import, run.
- `test/manifest.test.ts`, "a manifest says what each module exports":
  `main` by `start` alone.
- `test/architecture.test.ts`, "what a module exports an item as is
  exported_as's alone": `own_aliases` asked by the pipeline alone.
- Mutations: `own-alias-exported-by-name`, `own-alias-constant-exported-by-name`,
  `own-import-sorted-by-local`, `own-alias-importer-by-name`, `own-alias-of-another-module`,
  `own-alias-use-camel-case`, `own-alias-declared-exported`,
  `own-alias-reclaimed-unlisted` and the amended `own-alias-dropped`
  (`default_exports.rs`, `pipeline.rs`, `analysis/naming.rs`,
  `declarations.rs`, `prepare/reclaim.rs`).
