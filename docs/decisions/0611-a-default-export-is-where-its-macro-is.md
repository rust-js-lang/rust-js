# 0611. A default export is where its macro is

Status: Accepted. Extends [0192](0192-next.md).

Case ([0262](0262-when-rust-and-js-disagree.md)): A: the same function is
the module's default.

## Context

JS writes a module's default function two ways, both common: React's
`noop.js` is `export default function noop() {}`; react.dev's components
are `function Recap() { .. }` and, after it, `export default Recap;`. In
Rust each is a function and `js::export_default!(f);`, which was always
`export default f;` after the module's code. One of the two had no way to
be written.

## Decision

**The JS's `export default` is where the Rust's `js::export_default!` is**
(`bindings::default_declared`):

- written right after its function, nothing but comments between, of a
  function of no other export, not `pub`: that function's declaration,
  `export default function noop() {}`, in the `.d.ts` too, `export default
  function noop(): void;`;
- anywhere else, or of a `pub` function, which is exported by its name
  too: `export default recap;`, as before.

The user's decision: both are expressible, by where the macro is written,
as JS has them by where `export default` is.

## Consequences

- react.dev's ported components whose macro follows their function are
  `export default function` now; to keep `export default Recap;`, the
  macro goes after what follows it.

## What holds it

- `test/modules.test.ts`, "a default export is where its macro is", of
  each; `test/declarations.test.ts`, "declarations declare a private
  default export", its `.d.ts`.
- Mutations: never declared, declared after other items, declared of a
  `pub` one.
