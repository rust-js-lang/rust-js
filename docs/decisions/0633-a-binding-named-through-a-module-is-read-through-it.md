# 0633. A binding named through a module is read through it

Status: Accepted. Amends [0622](0622-a-global-binding-a-module-names-is-its-const.md);
extends [0100](0100-separate-crates.md) and [0240](0240-reexports.md). Amended by
[0773](0773-a-held-default-is-exported-once.md): a binding the default names is
exported as the default alone.

Case ([0262](0262-when-rust-and-js-disagree.md)): the program can't
observe it: the module's `const` is the JS value the binding reads, read
once where the module loads, as upstream's is.

## Context

React's packages read what they share through `shared`'s modules:

```js
// shared/assign.js
const assign = Object.assign;
export default assign;
// shared/ReactSharedInternals.js
import * as React from 'react';
const ReactSharedInternals = React.__CLIENT_INTERNALS_DO_NOT_USE_OR_WARN_USERS_THEY_CANNOT_UPGRADE;
export default ReactSharedInternals;
// react-reconciler
import assign from 'shared/assign';
import ReactSharedInternals from 'shared/ReactSharedInternals';
```

ADR 0622 made `shared/assign` a module's `const`, but a caller that named
it through the module, `use shared::assign::assign; assign(t, &[a])`, still
read the binding in place, `Object.assign(t, a)`, as its Consequences
said. And a binding's `static` couldn't be a module's default at all:
`js::export_default!(REACT_SHARED_INTERNALS)` was refused, and each read
of one was `react.__CLIENT_INTERNALS_..` in the caller's own module.

Rust resolves `shared::assign::assign` to the binding's `DefId`: the
module a caller named it through is in the path it wrote, which HIR keeps
(each segment's `Res`, and the `use` that brings a name in), not MIR.

## Decision

**A caller that names a binding through a module that holds it reads it
through that module**, as upstream does: it imports that module's export,
its default as a default import, `import assign from "./assign.js"`,
else the name it exports, and reads it by that name, called with the
binding's own arguments, a variadic's spread (ADR 0221). A module holds a
binding where it names it as a value of its own, its default or a `pub
use` (ADR 0622). A caller that names the binding itself,
`js::object::assign`, or a module that only declares it, reads it in
place.

```rust
use crate::assign::assign;

pub fn merged<'a>(target: &'a Unknown, source: &Unknown) -> &'a Unknown {
    assign(target, &[Some(source)])
}
```

```js
import assign from "./assign.js";

export function merged(target, source) {
  return assign(target, source);
}
```

**One place answers where a read goes**: `bindings::held` walks each
module once, what it holds (`HeldBinding`), and each path of each body
once: a path of more than one segment names the binding through the
module its last but one segment is; a name alone, through the `use` in
its module that names it, or a glob's module. What it finds is keyed by
the path's span, which MIR's constant and THIR's expression of it share,
and every read asks it through `FnCx::binding_ref`: a call (by its
callee's THIR expression), a binding taken as a `fn` (ADR 0602), a
static's read, and the analysis that names a module's imports, which
leaves out what's read through another module.

**A binding's static is a module's `const` as a global's function is**:
`js::export_default!(REACT_SHARED_INTERNALS)`, the naming form, as of a
global's function, or a `pub use` of one. Its `const` reads its path,
importing what that needs, a namespace import of `react#*`; one whose
path is a JS module's export itself, `react#Fragment`, is that module's,
re-exported by a `pub use` (ADR 0240). The `const` is named as any item
is (ADR 0573): its Rust name as written, `REACT_SHARED_INTERNALS`, or its
`#[rust_js::name]`, `ReactSharedInternals`, where JS's isn't derivable
from Rust's; its `.d.ts` declares it as its type.

```rust
// lib.rs
unsafe extern "Rust" {
    #[cfg_attr(rust_js, rust_js::name = "ReactSharedInternals")]
    #[link_name = "react#*.__CLIENT_INTERNALS_DO_NOT_USE_OR_WARN_USERS_THEY_CANNOT_UPGRADE"]
    pub safe static REACT_SHARED_INTERNALS: &'static Internals;
}
// internals.rs
pub(crate) use crate::REACT_SHARED_INTERNALS;

js::export_default!(REACT_SHARED_INTERNALS);
```

```js
import * as react from "react";

const ReactSharedInternals = react.__CLIENT_INTERNALS_DO_NOT_USE_OR_WARN_USERS_THEY_CANNOT_UPGRADE;

export default ReactSharedInternals;
```

**A `pub use` of a binding through a module that holds it is that
module's, re-exported**, `export { keys } from "./keys.js";`, by its name
and as the default where it's that too: not a `const` of its own, which
would read it in place.

**Across crates, the library's manifest says what its modules hold**: a
`held` entry each, the binding's and the module's keys, its name in Rust
there, the module's path, what it exports it by and its `const`'s name.
A consumer that names a binding through a library's module imports that
export from the library's JS, named after the `const`, `import
ReactSharedInternals from "../shared/internals.js"`. A manifest a compiler
wrote before has none, read as holding nothing.

## Why

- **It's upstream's JS**: react-reconciler imports `assign` and
  `ReactSharedInternals` from `shared`, and calls `assign(..)`.
- **It's the path the Rust wrote**: what a caller names through a module
  is that module's, as a function it names through one is imported from
  it; what it names itself is the binding's.
- **One place answers each question**: which module a read goes through
  is `bindings::held`'s, asked once, and the manifest carries the
  library's answer to its consumers as data, not worked out again.

## Alternatives

- **Every caller reads the binding in place**: 0622's, not upstream's,
  and a module's default nothing reads.
- **By the caller's module, from its `use`s alone**: a module that names
  a binding through a module and itself, inline, would read both through
  the module; the path is where each is named.
- **The wrapping form for a static, `js::export_default! { unsafe extern
  .. }`**: a second way to make the same `const`, as the naming form
  makes it. The naming form is the one a global's function has; a
  static only the macro names is what a module exports, `pub` where a
  crate's modules are `pub`, so a plain rustc reaches it.
- **Name a static's `const` after its module, or camelCase it**: no rule
  gives `ReactSharedInternals` of `REACT_SHARED_INTERNALS`; a variable's
  case (ADR 0573) gives `reactSharedInternals`, which isn't upstream's
  either. Its `#[rust_js::name]` says it, as any item's.
- **The library's consumer works out what a module holds from its
  metadata**: rustc's metadata of a module isn't the JS it exports, and
  ADR 0100 carries what crosses as the manifest's data.

## Consequences

- A namespace import of `react#*` is named as before, `react` (or the
  name of the one module that holds its bindings, ADR 0256), not
  upstream's `React`: no rule names a package's namespace as its users
  capitalize it, `React` but `path`, without a list of packages.
- A library's module a consumer names must be `pub` to Rust, so its
  `pub use` of a binding exported it by its name as well as its default,
  `export const assign = Object.assign; export default assign;`, as a
  `pub` function the macro names is (ADR 0611); upstream's has the
  default alone, as ADR 0773 now has it.
- A static only `js::export_default!` names, private and in a crate whose
  modules aren't `pub`, is dead to a plain rustc, as a private function
  the naming form names is (ADR 0611).
- A `pub use` of a binding through a library's module writes nothing, as
  a `pub use` of a library's function does; a default named through one
  is the module's own `const`.
- A path a macro writes is keyed by its span, as any: two paths at one
  span name the binding the same way.

## What holds it

- `test/lowering-bindings.test.ts`, "a binding named through the module
  that holds it is read through it": `assign` and a static by default,
  `keys` by `pub use` and re-exported through it, read by a call, a `fn`
  taken and a static's read, run in Node against a stand-in `react`; and
  a module that names each itself, in place.
- `test/cargo-link.test.ts`, "a library's module that holds a binding is
  what a consumer reads it through": the manifest's `held`, and a
  consumer's imports of `shared`'s modules, run in Node.
- `test/declarations.test.ts`, "declarations give a binding's static a
  module holds its type": `declare const iterator: symbol;`, a
  re-export's `.d.ts`, and TypeScript checks a caller.
- Mutations `held-read-in-place`, `held-name-alone-unfollowed`,
  `held-name-not-default`, `held-static-unheld`, `held-reexport-a-const`,
  `held-call-named-at-call`, `held-fn-pointer-in-place`,
  `held-static-in-place`, `held-read-imported-too`,
  `held-import-named-by-module`, `held-unlisted`, `held-static-typeof`.
