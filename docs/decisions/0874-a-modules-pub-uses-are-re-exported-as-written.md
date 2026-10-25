# 0874. A module's `pub use`s are re-exported as written, or refused

Status: Accepted. Amends [0240](0240-reexports.md), whose re-exports were
one statement for each module, ordered by its path, of the crate's
functions and bindings alone, and refused a JS module's namespace; and
[0596](0596-a-pub-use-of-a-modules-own-item-exports-it.md), whose private
`use .. as` named its import camelCase whatever its `#[rust_js::name]`
said.

Case ([0262](0262-when-rust-and-js-disagree.md)): N, of what JS can
import of a module and how its file reads; no Rust program can tell. A
`pub use` rust-js can't say is D, an error, where it was left out.

## Context

React's `ReactClient.js` is one list of what the package exports, in an
order its authors chose, with comments that say why each group is there:

```js
export {
  Children,
  ..
  ReactVersion as version,
  ReactSharedInternals as __CLIENT_INTERNALS_DO_NOT_USE_OR_WARN_USERS_THEY_CANNOT_UPGRADE,
  ReactCompilerRuntime as __COMPILER_RUNTIME,
  // Concurrent Mode
  useTransition,
  ..
  // DEV-only
  useId,
  act,
  captureOwnerStack,
};
```

Its port is a run of `pub use`s, with those comments. What rust-js made
of them:

- **A `pub use` of another crate's item re-exported nothing**, silently:
  `pub use shared::react_symbols::REACT_FRAGMENT_TYPE as FRAGMENT;`,
  `pub use shared::react_version::REACT_VERSION as VERSION;`. The loop
  that wrote re-exports looked for the crate's own functions alone.
- **Nor did a class's**, `pub use crate::react_base_classes::Component;`,
  **nor a module's**, `pub use crate::react_compiler_runtime as
  __COMPILER_RUNTIME;`, nor a binding a library's module holds,
  `pub use shared::assign::assign;`.
- **One of what another module re-exports named the module that
  declares it**: ReactServer's `pub use crate::react_cache_server::cache;`
  was `export { cache } from "./ReactCacheImpl.js"`, where upstream's is
  `from './ReactCacheServer'`.
- **Each module's names were one statement, the statements by module
  path**, and the comments before the `pub use`s were lost.
- **A private `use .. as`'s `#[rust_js::name]` was ignored**:
  `#[rust_js::name = "_jsxDEV"] use jsx_dev as _jsx_dev;` imported
  `jsxDev as _jsxDev`.

## Decision

**Each `pub use` of what has JS is re-exported, or refused**, never left
out. What it names, in the order the module writes them:

- **the crate's function or constant, from the module its path names**,
  which may re-export it itself, by the name it has there: `pub use
  crate::server::cache;` is `export { cache } from "./server.js"`, by
  `server`'s own `pub use`'s name where the `use` keeps it; through no
  other module, from the one that declares it, by what that one exports
  it as (`default_exports::exported_as`, ADRs 0596, 0773);
- **a library's function, constant or default, from its file**, by what
  the library's manifest says it exports it as: `export { LIMIT, helper as
  renamed } from "../dep/lib.js"`, `export { default as version } from
  "../dep/version.js"`; and a binding a library's module holds, from that
  module's file (ADR 0773);
- **a class, by its constructor** (ADR 0761), `export { Circle } from
  "./shapes.js"`, the crate's or a library's;
- **a module of the crate's, its namespace, `export * as
  __COMPILER_RUNTIME from "./runtime.js"`**, and a JS module's namespace
  a binding names, `#*`, too, which ADR 0240 refused as TypeScript's
  model had no `export * as`: it has now, `{ kind: "export", from,
  names: [["*", ns]] }`;
- **a type, a trait or a macro**, which has no JS, nothing, as before.

What has JS and can't be re-exported yet is an error: `pub use m::*;` of
a module with JS (`pub use` each by its name), a `pub use` of std's
function or constant, which no JS module of rust-js's holds, another
rust-js crate's module, whose namespace no file of this crate's is, a
class without a constructor, and a module's own class by another name.

**Each `pub use` is an `export { .. } from` where it's written**, in the
order the module writes them, the comments before it before it: the
re-exports are statements of the module, each at its `pub use`'s place
in the Rust, which the printer's comments are placed by (ADR 0444). A
`pub use` of a tree, `pub use hooks::{use_id, use_state};`, is one
statement of each module it names. **A run of them from one module, with
no comment or blank line between, but their attributes, is one
statement**: `export { useId, useCacheRefresh as unstable_useCacheRefresh
} from "./hooks.js";`, as a person lists a module's names once. They're
still after the module's imports and before its code, where a module's
Rust writes them.

```rust
// Hooks
pub use hooks::use_id;
#[cfg_attr(rust_js, rust_js::name = "unstable_useCacheRefresh")]
pub use hooks::use_cache_refresh as unstable_use_cache_refresh;
#[cfg_attr(rust_js, rust_js::name = "__COMPILER_RUNTIME")]
pub use crate::runtime as __COMPILER_RUNTIME;
// DEV-only
pub use server::cache;
```

```js
// Hooks
export { useId, useCacheRefresh as unstable_useCacheRefresh } from "./hooks.js";
export * as __COMPILER_RUNTIME from "./runtime.js";
// DEV-only
export { cache } from "./server.js";
```

**A private `use .. as`'s `#[rust_js::name]` is its import's name**, as
a `pub use`'s is its export's: `import { jsxDev as _jsxDEV }`. One name
attribute, one meaning, the name the `use` gives in JS.

## Why

- **What Rust makes public, JS can import**: a `pub use` left out is a
  module whose JS consumers find nothing, the silence AGENTS.md rules
  out; one that can't be said is an error that says what to write.
- **The module reads as it's written**: its order and its comments are
  the author's, `// Concurrent Mode`; a list sorted by path is the
  compiler's.
- **The path the program wrote is upstream's**: ReactServer re-exports
  `cache` from ReactCacheServer, the module its Rust names.
- **`export * as ns from` is one statement**, with no local name to keep
  apart from the module's own, where `import * as ns; export { ns }` is
  two.

## Alternatives

- **Upstream's one `export { .. }` list after the code**, every name
  imported first: ReactClient's form, but two statements for each name
  and a second form beside `export { .. } from`, which react.dev's
  `Challenges/index` and the scheduler's port read; and a module's
  `pub use` of a JS module's binding would import it to export it.
- **Each module's names as one statement, as before, in the order its
  first `pub use` is written**: a comment between two groups of one
  module's names has no statement to be before.
- **A statement for each `pub use`**: ReactClient's eleven symbols of
  `shared/ReactSymbols`, each `#[rust_js::name]`d, eleven lines of one
  module.
- **`export * from` for a glob**: it re-exports what the module exports
  for its crate's other modules too, its `pub(crate)` items, which Rust
  doesn't make public; refused until a decision says which.
- **Through a library's module as its path names it**: a library's
  manifest names the file each item is declared in, not its modules'
  re-exports, so a `pub use dep::reexported` is from the file that
  declares it.

## Consequences

- `export { helper, other as renamed } from "./inner.js"` of two
  adjacent `pub use`s is as before; a module whose `pub use`s of one
  module are apart, a comment or a blank line or another's between, now
  has a statement of each, in its order. A module's default of another
  module's function is re-exported where its `js::export_default!` is.
- A `.d.ts` re-exports as its JS does, `export * as ns from` too.
- A library's manifest says what its module re-exports, by name, as
  before (`output::exports`).

## What holds it

- `test/modules.test.ts`, "a module's pub uses are re-exported as
  written, with their comments": comments, order and runs, a class, a
  module's and a JS module's namespace, one through the module that
  re-exports it, a private `use`'s name, a module of only a namespace,
  in the JS and the `.d.ts`, run.
- `test/cross-crate-items.test.ts`, "a pub use of a library's item
  re-exports it from the library's file": a constant, a function, a
  default, a class and a held binding, run.
- `test/diagnostics.test.ts`, "a pub use of std's function" and "a pub
  use of every name of a module"; the guide's
  [limitations](../guide/limitations.md#modules).
- Mutations in `scripts/mutations/lower/reexports.ts` (each kind, the
  path's module, the name kept, the order, runs, comments, tree leaves,
  refusals), `to_oxc.ts` (`reexport-unprinted`,
  `reexport-comments-unplaced`), `to_oxc/statements.ts`
  (`namespace-reexport-as-a-name`), `analysis/naming.ts`
  (`use-name-attribute-ignored`).
