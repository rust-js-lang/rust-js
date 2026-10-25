# 0611. A default export the macro holds is the function's declaration

Status: Accepted; amended (a `pub` one, below; and what the naming form names, the default alone too). Extends [0192](0192-next.md).

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

## Amended: a `pub` one is the default alone, in its `.d.ts` too

A `pub fn` the macro held was `export default function noop` in its JS,
and `export function noop` in its `.d.ts`: TypeScript let a caller
`import { noop }`, which the JS doesn't export, and refused `import noop`,
which it does. The `.d.ts` asked the item's visibility where the JS asked
the macro.

**What `js::export_default! { .. }` holds is the module's default and
nothing else, whatever its visibility, in its `.d.ts` as in its JS**: a
function declared where it is, `export default function noop(x: number):
number;`, its doc comment its JSDoc; a constant declared, then exported,
`declare const VERSION: string; export default VERSION;` (ADR 0625).
`declarations::module` asks the one question the JS asks, whether the
macro holds it (`bindings::default_declared`), and declares each item
once, in the module's order; a private one held is declared there too,
no longer after the rest. The naming form, `js::export_default!(page);`,
is as before: `export default page;`, a `pub` one exported by its name
as well.

## Consequences

- A function a module's `js::export_default!(f)` names after it stays
  `function f() {} export default f;`, react.dev's components'.

## What holds it

- `test/modules.test.ts`, "a default export is the function's declaration
  where the macro holds it", of each form; `test/declarations.test.ts`,
  "declarations declare a private default export", its `.d.ts`, with
  `jsx!` in it.
- `test/declarations.test.ts`, "declarations declare a pub default
  export the macro holds as the default alone", a `pub fn` and a `pub
  const`, type-checked.
- Mutations: never declared, the function's own mark unseen; a `pub` one
  declared by its name (`pub-default-exported-by-name`), a held one not
  declared (`private-default-undeclared`) or not as the default
  (`private-default-exported`), a named one not declared
  (`named-default-undeclared`).

## Amended (2026-10-25): what the naming form names is the default alone too

Case ([0262](0262-when-rust-and-js-disagree.md)): N: no Rust program can
tell; what JS can import of the module changes, to upstream's.

React's `shared/reportGlobalError.js` is `const reportGlobalError = ..;
export default reportGlobalError;`. Its port, a `pub` thread-local the
react crate names, `js::export_default!(REPORT_GLOBAL_ERROR);`, was
`export const reportGlobalError = ..;` and `export default
reportGlobalError;`: one item exported twice, by two names, where every
importer, of its crate and of another, imports it as the default (ADR
0773, amended), and upstream exports it once.

**What `js::export_default!` makes the default, of either form, is the
module's default alone, whatever its visibility**: the naming form of a
`pub` function or thread-local of the module's own is `function page()
{ .. } export default page;`, `const reportGlobalError = ..; export
default reportGlobalError;`, declared so in its `.d.ts`, `declare function
page(): number; export default page;`. `pub` is how another crate names
it in Rust, as for a binding a module holds (ADR 0773); the module's
other items are as they were. The "as before" of the decision above, a
`pub` one exported by its name as well, is gone: two exports of one item
were two ways to import it, which [AGENTS.md](../../AGENTS.md) rules out.

Consequences: `examples/next/app/page.jsx`'s `Home` and a Next.js
`_app`'s `MyApp` are `function Home()`, exported as the default alone, as
create-next-app writes them. JS that imported such a function by its name
imports the default.

Held by `test/declarations.test.ts`, "declarations declare a pub default
export the naming form names as the default alone" (a `pub fn` and a `pub`
thread-local, their JS, `.d.ts` and TypeScript's check), `test/modules.test.ts`'s
Next.js route, and the mutations `named-default-exported-by-name`,
`named-default-constant-exported-by-name` (`pipeline.rs`) and
`pub-named-default-declared-by-name` (`declarations.rs`).
