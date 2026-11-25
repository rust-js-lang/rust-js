# 1204. What a crate offers JS is netted against Rust

Status: accepted (2026-11-25, the lead's decision on Tu's behalf, #2481)

## Context

83% of the JS boundary's P0s were found by hand (#2481's read of 455 P0s).
No net gave the compiled code JS's own values or asked whether what Rust
offers JS is there. #2544 was one: `pub mod sub { pub use dep as d; }` and
the root's `pub use sub as s;` compiled, and JS had no `s`, with nothing
said. The writer decided both what to write and what was written, so no
check could see the drop.

## Decision

**The build manifest says the paths a crate offers JS**, `offered_paths`,
an additive field (no version bump), public, which a host may read:

- each `pub` item of the crate's root and each of its `pub use`s, by the
  name the `use` gives it;
- through each module re-exported, the crate's own or a library's that has
  JS, by name or by a glob, what that module offers in turn;
- each with its Rust path, the path JS reads it by, and its kind: a
  function, a `const`, a `static`, a namespace or a type. A function has
  its parameters' and its result's types, a `const` or a `static` its own,
  as a tree of plain forms (`OfferedType`), any other by its Rust name.

Not a module's own `pub mod`: only Rust names it (#2403, lever (3)). Nor a
library's module of no JS, of types alone or a binding crate's.

**They're found of rustc's `module_children` alone** (`offered.rs`), apart
from what writes the JS and its `.d.ts`. Their record types are a module of
their own, `rust_js_program::offered`, which the front end writes and the
manifest carries, as neither phase's; `tooling/manifest.js` validates them. A path the writer drops is then a
path the JS lacks, not one both agree is gone.

**The export net** (`test/export-net.ts`, nightly) holds each crate's JS
and `.d.ts` to them:

- **existence:** each value path is in the JS, of its kind (a function a
  function, a namespace an object), a namespace where it offers a value;
- **declarations:** each value and type path is read through the `.d.ts`
  by `tsc`, which refuses one it lacks;
- **oracle A, native Rust:** each function of plain types, called with
  values at their types' bounds and inside (`test/generated-calls.ts`),
  and each `const` and `static`, gives what native Rust's does, printed
  alike by both sides (a float by its bits, a string by its code points);
- **oracle D:** no JS exception but a Rust panic escapes an export.

It runs over its own cases (`test/export-net/`), a crate alone and a
library with the app using it, and the crate pairs' table, each crate
compiled as a library with its declarations. A case of its own checks
something of each kind, so it never passes having checked nothing.

## Consequences

- The value generator and printers are one module, `generated-calls.ts`,
  which declarations-conformance reads its signatures through next, its
  regex parse of the source gone (#2573): one source of signatures.
- Later, each its own landing (#2481): oracle B (each result against its
  declared type), oracle C (counting proxies, volatile getters,
  `--js-reads`), binding stubs, the corpus as libraries; and a class's
  members, which v1 doesn't offer.

## What holds it

- `test/export-net.test.ts`: the own cases and the pairs; the offered
  paths of #2544's shapes, exactly.
- Put back by hand, #2544's bug fails the net: each path through a
  dropped re-export is missing.
- Mutations of `offered.rs`: `offered-namespace-unwalked`,
  `offered-alias-ignored`.
- Its first catch, on the crate pairs: a `pub use` of the crate's own
  static wrote no re-export, so a library root of a module and that `pub
  use` wrote no entry at all (#2581, fixed before this landed).
