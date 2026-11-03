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
