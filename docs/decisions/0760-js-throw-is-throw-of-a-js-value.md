# 0760. `js::throw(value)` is `throw value`, the one way to throw a JS value

Status: Accepted, by the user's decision of 2026-10-24. Extends
[0706](0706-panic-is-the-one-way-a-port-throws-an-error.md), whose
`panic!` stays the one way to throw a new `Error`, and
[0710](0710-every-unwinds-payload-is-the-js-error-thrown.md), whose
payload's `&JsError` is what this throws.

Case ([0262](0262-when-rust-and-js-disagree.md)): A. What's thrown is the
value itself, as Rust's unwinding has nothing to say of a JS value; what
the program observes of it, by `catch_unwind` or a JS `catch`, is that
value.

## Context

React's `lazyInitializer` (packages/react/src/ReactLazy.js) ends `throw
payload._result;`: what the loader rejected with, an `Error` or any value,
or the thenable it's still waiting on, which Suspense catches and waits
for. Its port couldn't write it: `panic!` makes a new `Error` of a message
(ADR 0706), and rustc refuses `resume_unwind` and `panic_any` of a JS
value, which isn't `Send`. The port had `todo!("throw payload._result")`.

## Decision

**`js::throw(value: Option<&Unknown>) -> !` is `throw value`**: a value
JS has, thrown as it is, `undefined` of `None`. A binding of the js crate,
`#[link_name = "throw"]`, `JsForm::Throw`: its call is the statement
`throw this`, and the call's value, which `!` has none of, is nothing.

```rust
pub fn lazy_initializer(payload: &Payload) -> Option<&'static Unknown> {
    if payload.status.get() == Status::Resolved {
        payload.result.get()
    } else {
        js::throw(payload.result.get())
    }
}
```

```js
export function lazyInitializer(payload) {
  if (payload._status !== "Resolved") {
    throw payload._result;
  }
  return payload._result;
}
```

- **The one way to throw a JS value.** `panic!` is the one way to throw a
  new `Error` (ADR 0706); `resume_unwind` of a caught payload throws it
  again, the same value. Each does a different job.
- **What it throws unwinds as any JS throw** (ADR 0660): it runs drops,
  and `catch_unwind` catches it, its payload's `downcast_ref::<&JsError>()`
  the value thrown, a thenable's or a number's too (ADR 0710), and its
  `downcast_ref::<String>()` none.

## Why

- **It's React's JS**, `throw payload._result;`, one statement where the
  port had none.
- **It's a JS operator, so it's a binding**, as `js::new` is `new` and
  `js::delete` `delete`: where Rust meets JS, in the js crate.
- **`Option<&Unknown>` is what JS's value is**: a property read,
  `js::get`'s, a cell of one, is `Option<&Unknown>`, so a value read is
  thrown as it is, `undefined` too.

## Alternatives

- **`panic_any(value)` or `resume_unwind(Box::new(value))`**: rustc refuses
  either of a `&Unknown`, which isn't `Send`; and either would read as a
  typed Rust payload, an `$Unwinding`, not the value.
- **A generic `throw<T: AnyShape<M>>(value: &T)`**: no `undefined`, and a
  second spelling of what `js::unknown(..)` already gives.
- **`panic!` of the value's text**: a new `Error`, not the thenable
  Suspense waits on.

## Consequences

- A value thrown by `js::throw` is no panic: it has no Rust message, and
  `catch_unwind`'s `String` downcast of it is `None`, as of any JS throw.

## What holds it

- `test/lowering-throws.test.ts`, run in Node: React's lazy payload, a
  thenable, a string, a number and an `Error` thrown as they are, and
  `undefined` of `None`; `catch_unwind` of each, its `&JsError` the value
  and its `String` none, and `resume_unwind` of it the same value.
- Mutations `js-throw-made-an-error`, `js-throw-unthrown`
  (`lower/binding_calls.rs`).
