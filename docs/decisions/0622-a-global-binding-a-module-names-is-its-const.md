# 0622. A global binding a module names is its `const`

Status: Accepted. Extends [0240](0240-reexports.md) and
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
