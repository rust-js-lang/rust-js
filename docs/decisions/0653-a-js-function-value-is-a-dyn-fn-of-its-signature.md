# 0653. A JS function value is a `dyn Fn` of its signature

Status: Accepted; amended by [0694](0694-typescripts-function-gives-any-value.md):
TypeScript's `Function` is `&dyn Fn() -> Option<&'static Unknown>`. Amends [0024](0024-web-crate.md)'s and
[0260](0260-webapi-callbacks.md)'s webapi generator, whose
function JS gives was a `&'static JsObject`, and
[0648](0648-v8s-stack-trace-api-is-the-node-crates.md), whose
`Error.captureStackTrace` is bound now, and `CallSite.getFunction()` a
`dyn Fn()`.

Case ([0262](0262-when-rust-and-js-disagree.md)): N: each is the JS
function it is, read, called and compared as JS does.

## Context

A function JS gives had two types. The js crate's, `Reflect`'s
descriptor's `get`, a state's setter (ADR 0367), node's
`Error.prepareStackTrace` (ADR 0648), were a `&'static dyn Fn` of their
signature, and `js::Kind::Function` a `&dyn Fn()`. webapi's generator
made every other one a `&'static JsObject`, "what it takes isn't said":
each event handler property, `button.onclick()`, and a queuing
strategy's `size`; node's `CallSite.getFunction()` followed it. A
`JsObject` can't be called, so a program that reads `onclick` to call it,
as React's event code reads a handler, can't; and two types for one
thing is a choice an agent gets wrong. `Error.captureStackTrace`'s
`constructorOpt?: Function` had no type, which ADR 0648 left unbound.

## Decision

The user's (2026-10-23): **a JS function value has one type, `&dyn
Fn(args) -> R` of its signature; TypeScript's `Function`, which says
none, is `&dyn Fn()`, as `js::Kind::Function` is.** Its `any` argument
is `Option<&Unknown>`, a value of any shape, and an `any` result
`Option<&'static Unknown>`.

| Binding | Was | Is |
|---|---|---|
| 766 event handler getters, `onclick` | `Option<&'static JsObject>` | `Option<&'static dyn Fn(&PointerEvent) -> Option<&'static Unknown>>` |
| of them `onbeforeunload` | the same | `.. dyn Fn(&BeforeUnloadEvent) -> Option<String>` |
| of them `onerror` | the same | `.. dyn Fn(EventOrStr<'_>, Option<&str>, Option<u32>, Option<u32>, Option<&Unknown>) -> Option<&'static Unknown>` |
| 2 `ByteLengthQueuingStrategy.size`, `CountQueuingStrategy.size` | `&'static JsObject` | `&'static dyn Fn()`, WebIDL's `Function` |
| node's `CallSite.getFunction()` | `Option<&'static JsObject>` | `Option<&'static dyn Fn()>` |
| node's `Error.captureStackTrace(target, constructorOpt?)` | unbound | `capture_stack_trace(&dyn Any, Option<&dyn Fn()>)` |

769 bindings moved, and one bound. The JS of each is what it was:
`button.onclick`, `callSite.getFunction()`.

- **A callback the generator gives is a `dyn Fn` of its WebIDL
  signature**, `&'static dyn Fn(..) -> R`, where its result was any. None
  is one yet, besides the handlers.
- **What's no function stays an object**: a callback interface,
  `walker.filter`'s `NodeFilter`, which TypeScript types as a function or
  an object of `acceptNode`; and `customElements.get(name)`'s
  `CustomElementConstructor`, a class, which `new` calls, TypeScript's
  construct signature: calling it would throw.
- **`std::ptr::eq(a, b)` of two is `a === b`**: a pointer to a `dyn Fn`
  is the JS function, as one to a JS object is the object (ADR 0285).
  Before, a `&dyn Fn` made a raw pointer and was refused, "addressable
  memory".
- **A `dyn Fn` is never falsy**, as a closure isn't (ADR 0298): whether
  a handler is set is `!!button.onclick`, as it was of the `JsObject`.
- **An event handler's setter keeps its closure**, `Option<Box<dyn
  FnMut(&E)>>`: a callback Rust gives JS to own (ADR 0260), not a value it
  reads.
- **Held by a test, not a type**: `JsObject` is still the type of a JS
  object. `test/webapi-coverage.test.ts`, "a function the webapi crate
  reads is a dyn Fn of its signature, not a JsObject", fails on any
  property TypeScript types as a function, `((ev) => any) | null`, that
  the crate reads as a `JsObject`, by its name. It fails on the crate as
  it was, `Document.onclick` first.

## Why

- **One type, the one rustc can call**: a function read is a function
  called, kept, compared, set back.
- **TypeScript's signature, as a binding types a value**: `onclick`'s is
  `(ev: PointerEvent) => any`.
- **`Function` as `Kind::Function` has it**: the one answer the js crate
  already gave to "a function of whatever it takes".

## Alternatives

- **`JsObject` for what JS gives, `dyn Fn` for what Rust gives**: two
  types of one value, and the value can't be called.
- **`fn` pointers**: a JS function may be a closure; no binding gives
  one.
- **A `Function` type of the js crate's, with `call`**: a second way to
  call a JS function, beside `f(x)`.

## Consequences

- **TypeScript types a queuing strategy's `size` as
  `QueuingStrategySize`, `(chunk) => number`**, where the generator reads
  WebIDL's `Function`: it's `dyn Fn()`, looser in what it gives. A
  `ByteLengthQueuingStrategy`'s chunk is an `ArrayBufferView`, which a
  `dyn Fn` can't take as an `impl`.
- **`Error.captureStackTrace`'s `constructorOpt` takes a function of no
  arguments and no result**, `&dyn Fn()`: a Rust function of others isn't
  one, as TypeScript's `Function` would take it.
- A handler called through the getter, `onclick(event)`, is
  `button.onclick(event)` where the read is folded into the call, `this`
  the button; called from a variable, `this` is `undefined`, as in JS.

## What holds it

- `test/webapi-coverage.test.ts`, "a function the webapi crate reads is a
  dyn Fn of its signature, not a JsObject"; "the webapi crate is what its
  generator writes".
- `test/bindings.test.ts`, "a function JS gives is a dyn Fn of its
  signature": an `onclick` kept in a thread-local, compared by
  `std::ptr::eq`, read and called, and set to a closure, each as JS and
  run.
- `test/node-bindings.test.ts`, "V8's stack trace hook is saved, set and
  restored": `Error.captureStackTrace(target)` and
  `Error.captureStackTrace(target, above)`, run in Node, and
  `callSite.getFunction()`.
- `test/snapshots.test.ts`, `web_forms`: `!!button.onclick`, as before.
- Mutations `dyn-fn-pointer-refused`, `dyn-fn-falsy`.
