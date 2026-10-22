# 0611. A default export the macro holds is the function's declaration

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

**`js::export_default!` of the function itself is its declaration as the
default**: `js::export_default! { fn noop() {} }` is `export default
function noop() {}`, in the `.d.ts` too, `export default function noop():
void;` (`bindings::default_declared`). **Naming one, `js::export_default!(recap);`,
is `export default recap;`**, as before, of a `pub` one too, which is
exported by its name as well.

The user's decisions: both are expressible, as JS has both; and by the
macro's form, which holds the function, so a plain rustc sees it used, no
`dead_code`, a generic one's too. An earlier draft read it from where the
macro was written, right after its function; a plain rustc then called the
function dead, as the `use` that names it isn't a use.

A `jsx!` in the function the macro holds is expanded as anywhere
(`jsx_syntax`, as a `thread_local!`'s).

## Consequences

- A function a module's `js::export_default!(f)` names after it stays
  `function f() {} export default f;`, react.dev's components'.

## What holds it

- `test/modules.test.ts`, "a default export is the function's declaration
  where the macro holds it", of each form; `test/declarations.test.ts`,
  "declarations declare a private default export", its `.d.ts`, with
  `jsx!` in it.
- Mutations: never declared, the function's own mark unseen.
