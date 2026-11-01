# 0622. A global binding a module names is its `const`

Status: Accepted; amended (the user's decision, 2026-11-02): a private `use` that renames a global is its `const` too. Extends [0240](0240-reexports.md) and
[0611](0611-a-default-export-is-where-its-macro-is.md); amended by
[0633](0633-a-binding-named-through-a-module-is-read-through-it.md): a
caller that names one through the module reads it through it, and a
binding's static is a module's `const` too.

Case ([0262](0262-when-rust-and-js-disagree.md)): A: what a module
exports, which a JS caller sees, and nothing a Rust caller does.

## Context

React's `shared/assign` is a global's function, exported:

```js
const assign = Object.assign;

export default assign;
```

In Rust a module names a global's function by its binding, `use
js::object::assign;`. `js::export_default!(assign)` was refused, as no
function of the module's own; a `pub use js::object::keys;` wrote
nothing, so a JS caller importing `keys` found nothing. A `pub use` of a
JS module's binding is `export { .. } from` it (ADR 0240); a global has
no module to re-export from. A `thread_local!` of a `fn` holding it,
`fn(&Unknown, &[Option<&Unknown>]) -> &Unknown`, is an arrow, `(target,
sources) => Object.assign(target, ...sources)`, as a call through a `fn`
pointer passes a slice as one array: a JS caller of it would pass an
array too, not Object.assign's arguments.

## Decision

**A global binding a module names as a value of its own, its default or
a `pub use`, is the module's `const` of the JS function as it is**,
exported where a `pub use` names it, by the name the `use` gives it:

```rust
use js::object::assign;

js::export_default!(assign);
```

```js
const assign = Object.assign;

export default assign;
```

`pub use js::object::keys;` is `export const keys = Object.keys;`. A
global is a binding that's a function, no method and no JS module's
import (`bindings::global_function`); `bindings::bound_globals` says
which a module names, and its JS and its `.d.ts` ask it.

**Its declaration is the global's own type**, `declare const assign:
typeof Object.assign;`, its overloads too, what TypeScript declares of
upstream's `const`. @rust-js/typescript's model gains the type
`{ kind: "typeof", name }`, TypeScript's type query, printed and read.

## Why

- **It's the JS a person writes**: the function as it is, which a JS
  caller calls with its own arguments, `assign({}, a, b)`.
- **It's Rust's**: what a `pub use` makes public, and what
  `js::export_default!` names, the module exports.
- **The global's type is TypeScript's**: a binding's Rust signature is
  one of `Object.assign`'s overloads, rest arguments of `unknown`.

## Alternatives

- **A `thread_local!` of a `fn`**: an arrow of Rust's arguments, which
  changes what a JS caller passes.
- **Refuse it**: upstream's module can't be written, and a `pub use` of
  a global stays nothing.
- **The binding's Rust signature as its type**: narrower than
  TypeScript's own declaration of the global.

## Consequences

- A call of it is its binding's, `Object.assign(a, b)`, where the caller
  names the binding itself; one that names it through the module reads
  it through it, `import assign from "./assign.js"`, and calls
  `assign(a, b)` (ADR 0633, which makes a binding's static a module's
  `const` as well).
- A `pub use` of a method's binding, `this.name(..)`, still writes
  nothing: it's no value a module can hold.

## What holds it

- `test/lowering.test.ts`, "a global binding a module names as its
  default or by pub use is its const": `assign` as a default, `keys` by
  `pub use` and as `names`, run in Node, and a method's `pub use`.
- `test/declarations.test.ts`, "declarations give a global binding a
  module names its global's type": TypeScript checks a caller of each.
- `test/typescript.test.ts`: `typeof Object.assign` read and printed.
- Mutations `bound-global-default-refused`, `bound-global-unexported`,
  `bound-global-renamed-lost`, `bound-global-of-a-method`,
  `bound-global-undeclared`.

## Amendment: a `pub use`'s `#[rust_js::name]` names it, renamed or not

react-art's ReactART.js exports ART's members by names of their own,
`export const Path = Mode.Path;` and `export {Transform}` of
`art/core/transform`'s default. The port's `#[cfg_attr(rust_js,
rust_js::name = "Path")] pub use art::mode::PATH;` printed `export const
PATH = Mode.Path;`, and the binding's default `export { default as
TRANSFORM }`: a `use` that keeps its item's name read only that item's
name, and one of a JS module's export only its Rust name, so the
attribute, which says what the `use` exports its item by, was dropped
unsaid. A wrong export name, which an importer of `Path` misses. **A
`use`'s `#[rust_js::name]` is the name it imports or exports its item by,
renamed or not**, one rule, `bindings::use_name`, which a held global's
`const`, a re-export of a binding's module export, of the crate's item and
of a class ask; without one, a `use` by the item's own name keeps the
item's JS name, and one renamed is its alias, camelCased (ADR 0240). Case
N: the name is what the program says. Held by
`test/lowering-binding-paths.test.ts`'s "a pub use of a binding is
exported by its name attribute" and the mutation
`use-name-attribute-dropped` (`lower/bindings.rs`).

## Amendment: a private `use` that renames a global (the user's decision)

The user's decision (2026-11-02, recommended in rd332's review, made while
the user was away by their standing instruction). react-art writes `const
slice = Array.prototype.slice;` and `slice.call(arguments)`, and React's
shared modules `const isArrayImpl = Array.isArray;`. A private `use
js::array::SLICE as slice;` was read in place, `Array.prototype.slice.call(..)`,
since a global became a module's `const` only through its default or a
`pub use`.

**A private `use` that renames a global binding, `use js::array::SLICE as
slice;`, is the module's `const slice = Array.prototype.slice;`, and each
read of it by that name there reads the `const`**: `slice.call(list)`,
`isArrayImpl(value)`. One by its own name, `use js::object::keys;`, or
`as _`, is read in place, as before; so is one of a JS module's import, an
import already.

Case ([0262](0262-when-rust-and-js-disagree.md)), stated as the user asked:
the `const` reads the global once, as the module loads, where the read in
place reads it at each call. They differ only where a program replaces the
global after the module loads, `Array.prototype.slice = ..` or a polyfill
installed later, which this ADR's `pub use` form already reads at load and
upstream's own `const` does. The user accepted the same trade for a
private `use` that names the `const` by renaming it.

- One owner, `bindings/held.rs` (moved out of `bindings.rs` for its
  cap): `held_of` holds the renamed global, and `named_through` reads a
  name a module's own renaming `use` gives through that module's `const`.
- No port module renames a global today (react-in-rust has no `use
  js::.. as ..` of a global); the corpus and the snapshots are unchanged.

Held by `test/lowering-binding-paths.test.ts`, "a private use renaming a
global binding is the module's const", run in Node; mutations
`private-renamed-global-unheld`, `renamed-global-read-in-place` and
`underscore-use-held` (`bindings/held.rs`).

## Amendment: a private renamed global is no part of the module's type (rd517, P3)

react-art's `use js::array::SLICE as slice;` printed `const slice =
Array.prototype.slice;`, right, and its `.d.ts` declared it, `declare const
slice: (value: number | null | undefined, ..) => ..`: by the binding's Rust
signature, which this ADR's Alternatives reject, and of a `const` no export
names, which a hand-written declaration leaves out. **A held global is
declared only where the module exports it, by name or as its default**; a
private one isn't, its type no part of the module's. Case N: declarations
only. Held by `test/lowering-binding-paths.test.ts`'s "a private use
renaming a global binding is the module's const", now with
`--declarations`; mutation `private-renamed-global-declared`
(`lower/declarations.rs`).

