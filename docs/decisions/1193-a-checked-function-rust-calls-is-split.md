# 1193. A checked function Rust calls is split: its JS entry and its body

Status: Accepted (2026-11-24, #2403 step 2b(a), the lead's approval of the
design). Amends [1192](1192-a-value-js-gives-is-checked-shallow-first.md):
a Rust caller no longer pays the checks; and
[0100](0100-separate-crates.md): a library's contract names each split
function's body, the library ABI 48.

Case ([0262](0262-when-rust-and-js-disagree.md)): none. What JS sees is
unchanged; Rust's calls skip checks of values Rust made.

## Context

ADR 1192 checks what a JS caller gives a function, first in its body. A
Rust caller pays the same checks of values its types already promise. In
the React port, every function 1192 checks has Rust-compiled callers:
546 functions are called from other port modules, with 796 of the 847
checks traced, and 51 more checks are on functions their own module calls.

## Decision

**A function whose parameters are checked, and which Rust calls or may
call, is two functions: its JS entry, by its own name, which checks and
calls its body, and its body, `f$`, which every direct Rust call calls.**

```js
export function double(n) {
  $checkU32(n, "double(n)");
  return double$(n);
}

export function double$(n) {
  return (n * 2) >>> 0;
}

export function quadruple(n) {
  $checkU32(n, "quadruple(n)");
  return double$(double$(n));
}
```

- **One owner, `boundary/split.rs`:** `Split`, the crate's split
  functions, each with its body's name. It's found once, the first time
  any lowering asks (`Lazy`, ADR 1148), or by the pipeline where none did.
  Three readers:
  - the pipeline writes each split function in two, after lowering
    (`written`);
  - the crate's own direct calls name the body (`crate_called_copy`);
  - the library's contract names it, `Item.unchecked`.
- **Which are split:** where Rust callers exist or may exist.
  - In a library, every function with a checked parameter, since its
    consumers may call any. Its body is exported.
  - In a program, those the crate's own bodies call directly. The body is
    private unless another module calls it, which imports it.
- **Which can't be:**
  - a method of a type's object;
  - a function that isn't at its module's top;
  - one that reads `js::arguments()` or `js::this()`, which its entry
    couldn't pass on;
  - one of rest parameters;
  - one with a parameter Rust takes apart in its pattern.
  These keep their checks first in their one function.
- **Only a direct call calls the body.** A function Rust gives as a value
  is the entry, `return double;`, as JS may call it.
- **A consumer's call:** a library's `Item.unchecked`, `"double$"`, reaches
  `Imported.unchecked`. Its import is planned with the library's other
  exports (`analysis`), and a direct call imports and calls it, read as
  the module reads its entry: `Dep.double$(n)` where the path names it
  through the library's module, `dep::double(n)`, as `Dep.double` is read
  (rd114), else `import { double$ as double } from "../dep/lib.js"`. Only
  where the module imports it by the file that exports the body: through
  the library's package root or a module that re-exports it, which export
  the entry alone, the call is the entry's, as is a call of a file's
  `default`. It isn't used for a function the consumer compiles itself
  (ADR 0914), or one its module reads through `require`.
- **TypeScript sees the entry alone.** The declarations are of the Rust
  items, so `f$` is never declared.
- **The library ABI is 48.** `library::VERSION`, `std/manifest.json` and
  `tooling/manifest.js`'s `LIBRARY_ABI` all move together, and a library
  compiled before is refused, as every bump refuses it.

## Why

- **Rust never pays for JS's checks.** A value a Rust caller gives is of
  its type already.
- **One function JS calls.** The entry keeps the function's name, its
  exports and its `.d.ts`, so nothing JS or TypeScript sees changes.
- **Values stay checked.** A function given as a value may reach JS,
  which calls it with anything.

## Alternatives

- **Checks in every function, unsplit** (ADR 1192 as it was): Rust callers
  pay, the port's 847 on its hottest paths.
- **Split everything, for uniformity:** a wrapper nobody calls, in every
  program, for functions only JS calls.
- **The body's name kept by every re-export:** a library's `pub use` and
  its package root carry `f$` too. Not yet: a consumer imports the body by
  the file that has it, and a re-exported path calls the entry, as before.

## What holds it

- `test/boundary-split.test.ts`, a library and its consumer:
  - the entry and its body;
  - a Rust call of the body;
  - a value kept as the entry;
  - the contract's `unchecked`;
  - the consumer's import and call of the body;
  - a program's function only JS calls left whole, and one Rust calls
    split, its body private;
  - the `.d.ts` declaring the entry alone.
- The mutations in `scripts/mutations/lower/boundary/split.ts`.

## Amendment: a namespace's method splits; a consumer checks a library's unchecked function (#2403, the lead's decisions, 2026-11-24)

A type's namespace's method JS may call splits as a function does: its entry, `Perms.fromBitsRetain`, and its
body, `Perms.fromBitsRetain$`, beside it in the object, which a Rust call calls, a library's through its object
by its manifest's `unchecked`. So a namespace's constant made by a call, bitflags' `READ`, and a Rust call
of a value Rust never branded, chrono's, pay no check. A library's manifest says whether its own JS checks each
item (`Item::checked`, library ABI 48 to 49). A consumer that gives JS one it doesn't, `pub use dep::m::f` by
name or by a glob, or a forwarder to it, writes its own checked function in its place, named apart where its
module imports the library's by that name, `function f$1(n) { .. }` and `export { f$1 as f }`. A
`pub use dep::m as ns` of such a module is a module of the consumer's in its place, `ns.js` beside the
module, `export * from` the library's file and a checked forwarder of each unchecked function by its own
name, which shadows the star's; the module's `export * as ns from "./ns.js"` (#2536, the lead's design,
ADR 1008 amended). Only where the library's module has such a function: else `export * as ns` of the
library's file, as before. Held by `test/boundary-reach.test.ts`, `test/glob-reexports.test.ts` and the
tree-shaking net (an unused member dropped, the shadowing forwarder's check kept, under Vite and Bun).

## Amendment: a body's import is named as its entry's (2026-11-25)

A consumer's direct call of a library's split function calls its body,
imported by its file, `f$`. That import was named apart from its entry's,
by the body's own name, so the module's `use` of it was lost: React's
port's `use react_server::react_flight_server::{create_request as
create_flight_request, ..}` read `import { createRequest$ as
createRequest$$1 }` beside the Fizz server's `createRequest$ as
createRequest`, 28 aliases lost and 130 names numbered apart over 13 files;
and a local named after the call it holds took the name the import left
free, ReactFiberRootScheduler's `const lanesToEventPriority =
lanesToEventPriority$(nextLanes)` where it read `const value`.

**A body's import is named as its entry's would be** (`library_imports.rs`,
`naming.rs`): by the entry's name, and by the module's `use .. as` where it
renames it, `import { double$ as twice }`. So a name is lost, or a local
renamed, no more than beside the entry before the split.
test/boundary-split.test.ts holds both, and mutations `split-body-named-apart`
and `split-body-unrenamed`.

Every module names every library export before lowering, the entry and its
body both, and the first named takes the word: the body is named first, as
a direct call imports it and only JS's use of the function imports the
entry. Named after it, a consumer's `Debug` that passes the library's
`quoted` on read `import { quoted$ as quoted$1 }` beside the entry's
`quoted`, which it never imports, where another function's local `quoted`
kept the import from reclaiming its word (test/crates-compiled-here.test.ts,
mutation `split-body-named-after-its-entry`). No temporary is named for an
entry whose body its item reads, the module's own or an import's, its own
name's too (`reclaim::entries_read`); no local reclaims the module's own
entry's word but its item's own, as before the split.
