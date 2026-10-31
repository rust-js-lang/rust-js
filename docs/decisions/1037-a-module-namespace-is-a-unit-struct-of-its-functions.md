# 1037. A module's namespace is a unit struct of its functions

Status: Accepted (the user's decision, rd292, option A). Extends
[0943](0943-an-object-of-methods-reading-this.md) (an object of methods,
whose `&self` methods are its module's functions too) and
[1018](1018-a-function-called-through-its-module-is-read-through-its-namespace.md)
(a module's namespace, `import * as M`, which a path through the module
reads).

Case ([0262](0262-when-rust-and-js-disagree.md)): none. Rust has one value
of a unit struct, so each call of its method is the one function; JS
reads it through the value it's given, which is the module's namespace,
whose member is the export's live binding, the function itself. Where JS
gives another instance of the module, Jest's `requireActual` of the
reconciler, the call is that instance's, as upstream's is: Rust can't
make one, so no Rust program tells the two apart.

## Context

React's noop renderer is given the reconciler's namespace and calls
through it:

```js
function createReactNoop(reconciler, useMutation) {
  ..
  const NoopRenderer = reconciler(hostConfig);
  .. NoopRenderer.createContainer(container, tag, ..) ..
}
```

Under Jest, `reconciler` returns `jest.requireActual('react-reconciler')`,
a fresh instance of ReactFiberReconciler, a module rust-js compiles,
whose host config is the object it's given. The port had no form for "a
module's namespace as a value": it declared a stand-in JS type
re-declaring each function the renderer calls (gap rd292, P1), a second
copy of the reconciler's signatures, and spread the noop's own
ReactFiberConfigNoop namespace, `...DefaultConfig`, through a binding of
`* as DefaultConfig` typed as any object.

## Decision

**A unit struct and its impl, each marked `#[cfg_attr(rust_js,
rust_js::namespace)]`, are the module they're in: the impl's `&self`
methods are the module's exported functions, and the struct's value is
the module's namespace** (`classes.rs`, `Member::Namespace`):

```rust
#[cfg_attr(rust_js, rust_js::namespace)]
pub struct ReactFiberReconciler;

unsafe impl js::Defined for ReactFiberReconciler {}

#[cfg_attr(rust_js, rust_js::namespace)]
impl ReactFiberReconciler {
    pub fn create_container(&self, container_info: &Container, ..) -> &'static OpaqueRoot {
        ..
    }
}
```

- **Each method is its module's function**, `export function
  createContainer(containerInfo, ..)`, with no `self` (a parameter
  nothing reads), declared so in its `.d.ts`. `naming.rs` names it a
  function of no type's object, as an object's method is.
- **A call on the namespace as Rust names it**, `ReactFiberReconciler
  .create_container(..)`, the struct's literal, and one in a method on
  its own `self`, is the function's, `createContainer(..)`, imported by
  its name, as React DOM imports it. **A call on any other value** is
  through it, `NoopRenderer.createContainer(..)`: the value JS gave,
  which may be another instance of the module. `crate_called_copy` tells
  them by the receiver: the namespace's literal lowers to an item
  (`FnCx::namespace_value`), its own method's `self` to nothing.
- **Its value is the module's namespace**, `import * as
  ReactFiberReconciler from "./ReactFiberReconciler.js"`, a library's
  imported from where the library's JS exports its functions, named
  after the struct; spread, `{ ...DefaultConfig }`, it is every export of
  the module. Its type is `typeof ReactFiberReconciler`, imported whole,
  `import type * as ReactFiberReconciler from ..`.
- **As a value**, a method is the arrow of the call through what it's
  given, `(a, b) => a.updateContainer(b)`.
- **Refused, each with what to write**: a struct with fields or type
  parameters (its value is the module, which holds nothing else); one
  that's a class or an object too; an impl in another module than its
  struct's (its functions are that module's); an item that's no method of
  `&self`; a method that reads `self` but to call another of its methods
  (the module has no value of its own to give); and the attribute on one
  of the struct and its impl alone.

## Why

- **One declaration**: the reconciler's functions, typed once by rustc,
  are what the renderer calls; the stand-in copy goes.
- **Upstream's JS**: React DOM's `createContainer(..)` and the noop
  renderer's `NoopRenderer.createContainer(..)`, each as its Rust names
  the receiver.
- **Beside its siblings**: a class (ADR 0761), an object of methods (ADR
  0943) and a namespace are each a struct and its impl marked, each
  method's JS what the mark says.

## Alternatives

- **A trait the reconciler defines, implemented by a unit struct whose
  dictionary is the module** (option B). A second declaration of each
  function, the trait's, and a new representation, a dictionary that's a
  module. Not taken.
- **Rust generics, `create_reconciler::<C: HostConfig>()`** (option C).
  Idiomatic, but the JS diverges from upstream's module mocking. Not
  taken.
- **Free functions beside forwarding methods.** A copy of each signature.
  Not taken.

## Consequences

- A module's `pub use` of another module's function is in its JS
  namespace but not a method of its struct: a renderer calling one
  through the value, React's `batchedUpdates`, needs it a method. Whether
  a namespace method that only passes its arguments on to another
  module's function is that function re-exported, `export {
  batchedUpdates } from "./ReactFiberWorkLoop.js"`, is open (rd292b).
- A crate's callers of a function made a method call it on the struct,
  `ReactFiberReconciler.create_container(..)`.
- A module that names its own namespace's value imports itself, `import *
  as Reconciler from "./reconciler.js"`, which ES modules allow; its
  declaration of its own type is `any`.
