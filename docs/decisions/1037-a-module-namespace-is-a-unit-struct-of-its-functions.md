# 1037. A module's namespace is a unit struct of its functions

Status: Accepted (the user's decision, rd292, option A); amended below
(rd292b, the user's decision: a method that only passes its parameters
on is the function it calls; its import named by a `use`). Extends
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
  through the value, React's `batchedUpdates`, needs it a method, which
  only passes its parameters on (rd292b, amended below).
- A crate's callers of a function made a method call it on the struct,
  `ReactFiberReconciler.create_container(..)`.
- A module that names its own namespace's value imports itself, `import *
  as Reconciler from "./reconciler.js"`, which ES modules allow; its
  declaration of its own type is `any`.

## Amendment: a method that only passes its parameters on is that function (rd292b)

The user's decision (2026-10-31). Case ([0262](0262-when-rust-and-js-disagree.md)):
none for a Rust caller, whose call of the method is the function's;
ADR 0262's exception of the forwarding family (rd196, ADR 0605) for a
JS one, which may tell them apart only by identity, now the function's.

React's reconciler re-exports the work loop's functions, `export {
batchedUpdates, .. } from './ReactFiberWorkLoop'`, which the noop
renderer calls through the namespace it's given. A Rust `pub use` puts
the function in the module's JS namespace, but no method on its struct
for `renderer.batched_updates(..)`. The port writes each as a method
that passes its parameters on:

```rust
pub fn batched_updates<A, R>(&self, f: impl FnOnce(A) -> R, a: A) -> R {
    react_fiber_work_loop::batched_updates(f, a)
}
```

**A namespace's method whose body only passes its parameters on, past
its `self`, in order, each as it is, to another of the crate's modules'
function is that function** (`passed_on::forwarded_function`, the shape
`body_queries::forwards_to` reads, ADR 0864's, of its parameters past
the first): its module re-exports it by the method's name, `export {
batchedUpdates, flushPendingEffects as flushPassiveEffects } from
"./ReactFiberWorkLoop.js"`, a run of them of one impl from one module
one statement (`reexports::forwarded_methods`), declared so; it's no
function of its own (`pipeline::reached::lowered_here`); a call of it in
its own module, which has no name of its own for it, is the function's;
elsewhere the name its module exports, as before; and a library's
manifest lists it, the module's re-exported names its exports
(`library::exported_names`).

It's that function only where the two take what JS gives them alike
(`pipeline::reached::forwarded`): its parameters' types the function's
(a `&mut` given as a `&` is a handle the function doesn't read), its
generic parameters given as they are (a fact or a drop the method knows
the function would be given), its caller's location both or neither,
the same bounds' dictionaries, and no copy its callers need (ADR 0051).
Each other is a function of its own.

**A namespace's import is named as the `use` that renames its struct
names it**, `use crate::react_fiber_config_noop::ReactFiberConfigNoop as
DefaultConfig` `import * as DefaultConfig`, upstream's createReactNoop's
(`analysis::namespace_in`, ADR 0202's every import by its `use`'s name).

Held by `test/namespaces.test.ts`: the re-export and its declaration,
the calls in and out of its module and through a value, run; each that
isn't one, run; a library's consumer's call; the import named by its
`use`. Mutations `forwarded-function-self-passed`,
`forwarded-function-of-its-own-module`,
`forwarded-function-other-generics`,
`forwarded-function-other-parameters` (`passed_on.ts`),
`forwarded-caller-location-unasked`, `forwarded-bounds-unasked`,
`forwarded-copies-unasked`, `forwarded-method-lowered`
(`pipeline/reached.ts`), `forwarded-methods-unexported`,
`forwarded-methods-each-a-statement` (`reexports.ts`),
`forwarded-called-in-its-module-by-its-name` (`mir/calls.ts`),
`forwarded-method-declared` (`declarations/namespaces.ts`),
`reexported-names-unlisted` (`library.ts`) and
`namespace-import-unrenamed` (`analysis/namespaces.ts`).

