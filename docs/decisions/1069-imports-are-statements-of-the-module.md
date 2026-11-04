# 1069. Imports and re-exports are statements of the module, where their `use`s are

Status: Accepted (rd637 and rd652, design (b), agreed by m6-wife and
m6-react). Amends [0865](0865-a-blank-line-is-where-the-rust-has-one.md),
whose rule now places every blank line of a module, its imports' too, and
[1063](1063-imports-are-in-the-order-of-their-uses.md), whose imports
were one block of text above the code.

Case ([0262](0262-when-rust-and-js-disagree.md)): N, none. ES hoists
imports and re-exports: a module's requested modules are linked and run
before any of its statements, in the order its `import` and `export ..
from` declarations are written, wherever they are among its code. Where
one is printed among the statements changes nothing a module does; their
order among themselves, the load order ADR 1063 decides, is kept exactly.
Blank lines run nothing.

## Context

A module's header was plain text, built in `to_oxc/header.rs` above the
code's program: rust-js's line, the file's leading comments, its
directives, then every import and re-export, one block, each after the
comments of its `use`. The code was a separate oxc program printed after
it, its map's lines shifted by the header's. So two authorities laid a
module out:

- **Blank lines.** Between code statements, `to_oxc::apart` put one where
  the Rust has one (ADR 0865); among the imports `format::spaced` kept none
  ("imports stay one block"), and the header added one after them. React
  DOM's ReactDOMClient groups its imports and re-exports, a blank line
  between groups, as its port's `use`s are grouped; the JS printed them as
  one block (rd637).
- **Placement.** An item written between two `use`s printed after every
  import: React's ReactFizzConfigMarkup declares `export const
  isPrimaryRenderer = false;` and `supportsClientAPIs` between two of its
  imports, as its port declares the `const`s between two `use`s, and the
  JS had them after all of them (rd652).

## Decision

**Each import and re-export is a statement of the module's program, at
the place the source names it, with its `use`'s comments before it**
(`to_oxc/header.rs`'s `entries`, `Cx::entry`, `among`):

1. **The order is ADR 1063's**, unchanged: each where the first `use` that
   makes it is, or the binding the module declares of it, or where a
   re-export is placed (`Reexport::at`), by specifier within one `use`;
   then those the source names neither way, packages then relative
   modules; then imports for their effect; then `@rust-js/runtime`'s
   helpers. One owner, `entries`, gives the order, and the statements and
   where comments go are made from it.
2. **Each is among the code's statements after those written, all of their
   Rust, before its place in its file**: a `const` between two `use`s is
   between their imports; a `use` inside a function is right above the
   function, JS having no import inside one. One placed nowhere, the
   fallback's, is right after the one before it, wherever that is, so the
   order of the loads is never changed by where they print.
3. **Its statement's span is the `use` that places it**, so the map places
   the import at the `use`, its comments are before it once, above the first
   of two one `use` makes (`header::commented_at`), and `apart` gives the
   blank lines before and after it as it does every statement's: one where
   the Rust has one between two `use`s or before the comments above the
   later, none where it has none, a multi-line `export {..} from` among the
   imports too. An import placed at a binding's declaration, which is no
   `use`, has a span of no width where the declaration starts: the map
   places nothing there, and the blank lines between function items tell
   nothing of the imports between them.
4. **Where the Rust tells nothing**, of an import no `use` places, the
   runtime's helpers, `format::spaced` keeps two imports or re-exports one
   block, as before, and sets one apart from the code before or after it,
   as a person does, before a comment above that code too.

What stays header-only, plain text above the program: rust-js's line, the
file's leading comments, the module's own `//!`s (`header::Docs`, #486)
and its directives (`header::header`). A blank line follows them, but
where the Rust has none right after the `//!`s, or where leading comments
are right above the code of a module that imports nothing (#419). The `#[test]` runner's imports, a
text file of its own, keep `import_line`.

## Why

- **One authority per rule** (AGENTS.md). A blank line and a place among
  the statements were each decided twice, once for the header's text and
  once for the code's program; now `apart` and the statements' order
  decide them for both.
- **The JS reads as the Rust is laid out**: a port whose `use`s are grouped
  as upstream's imports, with an item among them where upstream has one,
  prints them so.
- **Correctness is untouched**: no binding, specifier or order changes,
  only where the hoisted declarations print and the blank lines between.

Rejected:

- **(a) Keep the header text and teach it the Rust's blank lines.** A
  second copy of `apart`'s rule, over text, and an item between `use`s
  still after every import.
- **Every import at the top, in their order.** Keeps the second authority,
  and rd652's `const` still moves.
- **The fallback's imports first.** They'd print at the top where a
  re-export or `use` is written after code, but their loads would move
  before that one's, a change of the load order ADR 1063 decides (case C):
  not taken without its own decision.

## Consequences

- Imports print where their `use`s are, with the Rust's blank lines among
  them. Snapshots, corpus cases, the guide's examples and the React port's
  headers gain the blank lines between `use` groups and lose the one the
  header added where the Rust has none after its last `use`.
- A forwarder's re-export, placed at its function where no `use` names its
  module (ADR 1064), prints where the function is written, and the
  fallback's imports after it: the `modules` snapshot's `export { mixed,
  shadowed } from` and its namespace imports are at its end, after the
  functions above it.
- The map places each import and re-export at its `use`.
- In React's port, 174 generated files change, 156 of them by blank lines
  alone: ReactFizzConfigMarkup's `const`s are between its imports, 89 lines
  from upstream to 72, and ReactDOMClient's groups are upstream's. Where a
  port's `use` groups aren't upstream's import groups, its JS now shows it,
  ReactFiberBeginWork's blank lines between `use`s upstream has none
  between: the port regroups its `use`s, the work ADR 1063 already asked.
  Scheduler's and ReactServerStreamConfigEdge's `pub use`s, written last,
  print last, as upstream's `export`s are, the runtime's helpers after them.

- The user's decisions (2026-11-03), both to keep what this ADR prints:
  an import no `use` places, the runtime's helpers' and a fallback's,
  stays right after the import before it, even at a file's end, so the
  modules load in ADR 1063's order; and a `use` written right above code,
  with no blank line, prints its import with none before that code, as
  the Rust is written.

## What holds it

`test/lowering-comments-items.test.ts`'s "the header's blank lines are
the Rust's" (rd637) and "an item between two uses is between their
imports" (rd652), `test/modules.test.ts`'s re-exports, with the tests ADR
1063 and its amendments name for the order, now of `entries`; snapshots `imports`, `modules` and `playground`.
Mutations in `to_oxc/header.ts` and `format.ts`.

## Amendment: a `js::import!`'s comments are above its import (rd666)

React DOM's ReactDOMServerExternalRuntime writes `// Imports are resolved
statically by the closure compiler in release bundles ..` above `import
'./fizz-instruction-set/ReactDOMFizzInstructionSetExternalRuntime';`, as
its port writes it above its `js::import!(..)`. The comment was lost: an
import for its effect had no span (`Package::span` `NONE`), so no comment
was before it, and the `js::import!`'s `const _` writes no JS of its own
that one could be before.

Its span is now its `js::import!`, as a `use`'s import's is its `use`
(`pipeline/packages.rs`'s `of_module`), the one path a `use`'s comments
take: `commented_at` attaches the comments above it to its statement,
and `apart` sets it apart as the Rust has it. It's still placed nowhere
(`Package::named_at` `NONE`), so it's where rule 1 orders it, after the
named imports, right after the import before it: where it loads is
unchanged, wherever it's written. Case N of
[0262](0262-when-rust-and-js-disagree.md): a comment runs nothing.

One printed after code written after it, the vite-react example's
`js::import!("./App.css");` above its `use`s and TodoMVC's two, follows
an import whose `use` is below it: the blank line above it in the Rust
isn't one between them, so `apart` gives it none (`to_oxc.rs`), and the
imports stay one block, as rule 4 has it and create-vite's `App.jsx`
writes them: an import after one whose statement starts below its own
top. One in the Rust's order, after the `use` before it, has the blank
lines the Rust has between them, and the two of one `use`, `use
crate::api::{self, Failure};`, the one above it, before the first.

Held by `test/lowering-comments-items.test.ts`'s "the comments above a js::import! are
above its import", with mutations `js-import-comments-lost` and
`js-import-placed-where-written` in `lower/pipeline/packages.ts`, and
`import-after-later-code-keeps-blank` and `import-blank-of-one-use-dropped`
in `to_oxc.ts`.

## Amendment: each `use` of a module is an import of its own, a run of them one (rd649, the user's decision)

Upstream imports one module twice in four of the React port's files.
ReactFiberCommitWork has `import {..} from './ReactFiberLane'` in its
first lines and `import {clearTransitionsForLanes} from './ReactFiberLane'`
about 180 lines later. ReactFiberRootScheduler imports
`./ReactFiberAsyncAction` twice, ten lines apart, and ReactFiberReconciler
imports `./ReactFiberLane` under two specifiers. Rule 1 made one import of
a module, at the first `use` of it, so the port's second `use` printed
nothing where it's written: the linker grouped the crate's imports by path
(`link.rs::resolve`), and the packages were grouped by specifier
(`pipeline/packages.rs::of_module`).

**Each `use` of a module is its import, where it's written, and a run of
them of one module, with only attributes between, is one import at its
first.** This is the rule a run of `pub use`s already follows for one
re-export (ADR 0874, `reexports::assembled`), and they share
`reexports::only_attributes`. A blank line, a comment, another module's
`use` or an item between two `use`s ends the run. `UsesWritten` keeps
which statement follows another with nothing but attributes between
(`joined`). `written_at` gives the import's `use`: the `use` naming the
item, itself or through a module that holds or re-exports it, moved back
over the run's earlier `use`s that name something of the same module. The
crate's imports ask `module_import_at`, with the item each request is of
(`ImportTarget`'s last field), and the linker groups them by path and
`use`. The packages ask `export_at` and are grouped by specifier and
`use`. What no `use` names goes in the import at the module's first `use`:
of the crate, a function read through the namespace or a copy
(`module_at`); of a package, a binding the module declares itself, whose
import is the only one ordered by `package_named_at`. A namespace is at
the `use` of its module. `through_reexports` finds a re-exported item in
whichever of a file's imports holds it. The manifest still lists each
imported file once.

A run is one import because the user's c23b decision (ADR 0874, amended)
lets `#[rust_js::name]` name only a `use` of one name, so a module's
renamed imports are a run of `use`s. ReactFiberConfigDOM's `isEnabled as
ReactBrowserEventEmitterIsEnabled, setEnabled as ..` is one import in
upstream and stays one. That's also the form rustfmt keeps, since it
sorts a group's `use`s and leaves them apart.

Case ([0262](0262-when-rust-and-js-disagree.md)): N, none. ES modules
evaluate a module once, and the first import of it, which decides when it
loads, is where it was. So neither the load order nor any binding changes.

Rejected: (B) one import per `use`, a run too. It would give
ReactDOMComponent's two adjacent `import {..} from
'shared/CheckStringCoercion'`, but every run of named single-name `use`s
would print as one import per name, which the port couldn't say otherwise.
The user accepted ReactDOMComponent's single import by name.

Held by `test/lowering-comments-items.test.ts`'s "two uses of one module
are two imports, each where its use is" (a crate's module and a binding's
JS module, a namespace, and a comment ending a run), and by
`test/modules.test.ts`'s renamed and `#[rust_js::name]` runs, which stay
one import, and by `test/cargo-link.test.ts`'s run of a binding a crate's
module holds and that module's own function, `import merge, { size }`. Mutations are in `lower/analysis/naming.ts`,
`lower/pipeline/packages.ts` and `link.ts`.

## Amendment: a binding's comments are its import's (rd752)

React Native's ReactFeatureFlags.native-fb writes

```js
// Re-export dynamic flags from the internal module.
// Intentionally using * because this import is compiled to a `require` call.
import * as dynamicFlagsUntyped from 'ReactNativeInternalFeatureFlags';
const dynamicFlags: DynamicExportsType = (dynamicFlagsUntyped: any);
```

and its port writes the comments above the binding whose JS is that
import, `#[rust_js::link_name = "get ReactNativeInternalFeatureFlags#* as
dynamicFlagsUntyped"] fn dynamic_flags_untyped() ..`, and the
`thread_local!` of `dynamicFlags` right after it. The JS lost both
comments, and a blank line set the import apart from the `const`. Rule 3
gave an import placed at a binding's declaration no `use`, so no comments
(`Entry::commented` `NONE`), and a span of no width, which tells
`apart` nothing, so `format::spaced` set it apart from the code. Nor was
any comment above another binding kept, a global's, `JSON.parse`'s in
TodoMVC, or a second of one module, `./greet.js#default` in the `imports`
snapshot: a comment is before the next start only past what has no JS, a
type alias or a `use`, and a function item's body ended that.

**A binding's comments are the import's that's placed where it's
declared, a doc comment's JSDoc too, through the one path a `use`'s take**
(`header::entries`, `commented` the declaration where no `use` makes the
import; `commented_at`). **The code right after the binding is right after
the import, as the Rust has it**: of an import placed at a binding, the
code after it is after the binding's whole item, which `UsesWritten`
keeps now, not its signature (`header::declared_at`, `declared_end`).
Rule 3 still holds where it said something: between two imports placed at
bindings, and before their comments, the blank lines between function
items tell nothing, so the imports stay one block, as the playground's
CodeMirror bindings' do. **A comment above a binding of no JS where it's
written, a global's or one whose import is placed at another binding, is
before the next item that has JS, as one above a type alias is, and its
doc comment is nowhere**, a type alias's too (`js::Module::bindings`,
`to_oxc/starts.rs`'s `past_bindings` and `of_no_js`, a type alias's doc's rule moved there with it). No comment is
dropped.

Case ([0262](0262-when-rust-and-js-disagree.md)): N, none. A comment and a
blank line run nothing; no import, binding or load order changes.

The other ways: keeping the import at the top and the comments at the
binding, which upstream writes nowhere; or the comments of every binding
of one import above it, which moves a comment written far below above the
first binding's import (the reconciler's ReactFiberConfig bindings).

What changes: the `async`, `imports`, `playground` and `todomvc` snapshots
gain their bindings' comments, each above its import or the next item, and
the guide's bindings page its `put` binding's JSDoc above `import { put }
from "my-store";`. In React's port, 47 lines of the port's own comments
above bindings, most of them the reconciler's notes of what a
ReactFiberConfig binding is of, are now in its JS, where they were lost.

Held by `test/lowering-comments-items.test.ts`'s "a binding's comments are
above the import it makes", run in JS, `test/modules.test.ts`'s "imports
are where their bindings are declared" and `test/compiler.test.ts`'s
`imports` leaf; mutations `binding-import-comments-lost`,
`binding-import-blank-lines-unquieted` and
`binding-import-end-before-an-import` (`to_oxc/header.ts`),
`binding-import-blank-before-comments` and
`binding-import-code-after-spaced` (`to_oxc.ts`),
`binding-of-no-js-comments-lost`, `binding-of-no-js-doc-kept` and
`binding-import-doc-dropped` (`to_oxc/starts.ts`, where `doc-of-no-js-kept`
moved), and
`binding-declared-signature-only` (`lower/analysis/naming.ts`). The
module directives' collection moved to `lower/bindings.rs`'s
`module_directives`, beside `directives_of`, and a doc comment's of no JS
to `to_oxc/starts.rs`, so `lower/pipeline.rs` and `to_oxc.rs` stay within
the compiler's file cap; their mutations moved with them.
