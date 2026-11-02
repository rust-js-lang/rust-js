# 1064. A function that only passes its parameters on to one instantiation is that function, re-exported

Status: Accepted. The user's decision (2026-11-02, rd575: "forwarder is a
re-export"). Extends [0240](0240-reexports.md) and
[0874](0874-a-modules-pub-uses-are-re-exported-as-written.md), a `pub
use`'s re-export, and [1037](1037-a-module-namespace-is-a-unit-struct-of-its-functions.md)'s
rd292b amendment, a namespace's method that only passes its parameters
on.

Case ([0262](0262-when-rust-and-js-disagree.md)): none for a Rust caller,
whose call of the function is the call it makes, of the same JS function
given the same arguments. ADR 0262's exception of the forwarding family
for a JS caller (listed in its Consequences and the guide's differences,
`semantics.md`, 45): only the function's identity, its `name` and its
`length` tell the two apart, now the function's.

## Context

React's server entries re-export ReactFlightActionServer's
`decodeAction` and `decodeFormState`, generic over the bundler's server
manifest:

```js
import {decodeAction, decodeFormState} from 'react-server/src/ReactFlightActionServer';
export {.., decodeAction, decodeFormState, ..};
```

The port's `pub use react_server::react_flight_action_server::{decode_action,
decode_form_state};` printed `export { decodeAction, decodeFormState }
from 'react-server/src/ReactFlightActionServer'`, upstream's JS, but its
`.d.ts` re-exported the generic declaration, `decodeAction<M>(body:
FormData, serverManifest: M)`, which takes any manifest where upstream's
webpack entry types webpack's. The idiomatic Rust of "this function, at
webpack's manifest" is a function that passes its parameters on:

```rust
pub fn decode_action(
    body: &'static FormData,
    server_manifest: &'static ServerManifest,
) -> Option<Promise<&'static dyn Fn() -> Option<&'static Unknown>>> {
    react_flight_action_server::decode_action::<ServerManifest>(body, server_manifest)
}
```

which printed a second function, `function decodeAction(body,
serverManifest) { return decodeAction$1(body, serverManifest); }`, of an
import renamed around it.

## Decision

**A function whose body only passes each of its parameters on, as it is,
in order, to a plain function at one instantiation, giving back what that
gives, is that function re-exported by its own name**, its declaration
its own, at that instantiation:

```js
export { decodeAction, decodeFormState } from "react-server/src/ReactFlightActionServer";
```

```ts
export function decodeAction(body: FormData, serverManifest: ServerManifest): Promise<..> | null | undefined;
```

`export { decode as decodeReply }` where the names differ, by the
function's `#[rust_js::name]` too; the crate's function from its module,
a library's from its file, as a `pub use` of it is (ADR 0874,
`reexports::forwarded_from`). A run of them from one module, nothing but
whitespace and attributes between, is one statement, as a namespace's
forwarded methods are (rd292b), where it's written among the module's
`pub use`s (`reexports::assembled`, the one order of a module's
re-exports). What only it read, its import of the function or a module's
namespace, isn't imported.

Each condition holds of the Rust and of its JS, the one place each is
known:

- **The Rust** (`passed_on::forwarder`): a free function of no type or
  const parameters, its body that call alone, each argument a parameter,
  reborrowed at most, `Some(x)` none; the callee a plain function, not a
  method, a closure, a constructor or a binding, of another module of the
  crate's or a library's; its parameters' and result's types the
  callee's at what it's given, so the call converts nothing. A generic
  function that passes its type parameters on is a `pub use`'s to say.
- **What the callee's JS takes** (`pipeline::forwarders::found`): its
  own JS, not a copy its callers need that boxes what they give (ADR
  0051), nor one the crate compiles of a library's (ADR 0914), which is
  no item of the library's; no drop its call may leave out at its end
  (ADR 0098) and no `js::arguments()` read (rd332), so it takes what the
  forwarder's JS callers give it, no more.
- **Its module**: no body of it names the function, which has no name
  there once it's a re-export; not its default (ADR 0611), nor one a
  `pub use` of its own module renames (ADR 0596).
- **Its JS** (`passed_on::forwarded_call`), lowered as every function
  is: the call alone, `return f(a, b);`, or `f(a);` of what gives
  nothing, each parameter given in order and nothing else, of a function
  it names. Any conversion the call needs is JS of its own, a box, a
  dictionary, a drop, `$unwrap`, a `?? null`, and keeps the function; so
  does `#[inline(never)]` (ADR 0649), a comment in its body, which a
  re-export has no place for, and a function its module doesn't export.

Otherwise it's a function, as before.

## Why

- **It's the user's decision**, and upstream's JS: the entry re-exports
  the function, and its types say the bundler's manifest.
- **Its JS is what proves it**: what a call prints is lowering's to say,
  through the call's every rule, its boxes, its dictionaries, its
  copies. A forwarder whose JS is the call alone is the function by what
  it prints, not by a second list of those rules; what JS can't show, a
  copy's name, a drop left out, `arguments`, is asked of its owners.
- **One owner of a re-export**: a forwarder's names go through
  `reexports.rs`, as a `pub use`'s do, in one order, one statement of a
  run.

## Alternatives

- **The `pub use` of the generic function, with a `.d.ts` written for
  it**: `#[rust_js::types]` on a `use`, a second way to type an export.
- **Deciding before lowering alone, as rd292b's methods are**: a
  forwarder's call converts what it's given by rules of many owners, a
  generic `&mut T`'s box, a `fmt::Result`'s `$writeTry`, a `Result`'s
  `$tried`; a list of them here would be a second owner of each.
- **A forwarder its module calls, its calls the function's**: rd292b's
  methods are, of the crate's own function; a library's would need an
  import the analysis didn't ask. Kept a function, which reads as before.

## Consequences

- React's server entries write the forwarder at their bundler's
  manifest, and their `.d.ts` say it.
- A JS caller of such a function sees the function itself, `===` it,
  `name` and `length` its (ADR 0262's exception, `semantics.md`, 45).
- A `.d.ts` declares the forwarder as written; only its JS re-exports.

## What holds it

- `test/declarations-libraries.test.ts`, "a function that only passes
  its parameters on to one instantiation is its re-export": a library's
  forwarder and an app's, a run of three one statement, the `.d.ts` at
  the instantiation checked by TypeScript, run in Node, each the
  function; and each that stays a function: reordered, `Some(..)`,
  another statement, a drop given, a dictionary given, a box, a value
  discarded, a generic one, a library's function the crate compiles
  itself, one that takes a drop its call leaves out, one that reads
  `arguments`.
- `test/lowering-forwarding.test.ts`, "a function that only passes its
  parameters on to another module's is its re-export": the crate's, by its
  `#[rust_js::name]` too, imported by another module from it; and kept:
  one its module calls, a default, one renamed by its module's `pub use`,
  `#[inline(never)]`, one with a comment, its own module's, a method's, a
  binding's, a copy's, one that reads `arguments`.
- Mutations in `scripts/mutations/lower/passed_on.ts` (`forwarder-*`,
  `forwarded-call-*`), `pipeline/forwarders.ts` (`forwarders-*`,
  `forwarder-default-taken`, `forwarder-alias-taken`,
  `forwarder-declared-as-reexport`), `pipeline/packages.ts`
  (`forwarded-imports-kept`), `reexports.ts`
  (`forwarded-functions-each-a-statement`) and `pipeline.ts`
  (`type-reexport-an-import`, of the re-exports' names now).
