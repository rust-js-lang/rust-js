# 0692. A thrown `Error` is a panic's message and what JS threw, both

Status: Accepted. Amends [0660](0660-catch-unwind-catches-what-js-throws.md),
whose thrown `Error` of JS's own constructor was a panic alone, its
payload its message, and whose panic resumed was a new `Error`.

Case ([0262](0262-when-rust-and-js-disagree.md)): B, a payload of a panic
that downcasts as a `&JsError` too, where Rust's is a `&str` or a `String`
alone: rare, a program that tells a panic's payload from a `&JsError` by
its type and not its text, and harmless, as what it finds is the panic's
`Error`, its `message` the panic's.

## Context

React's `DetermineComponentFrameRoot` calls a component that throws, and
reads where it threw:

```js
} catch (sample) {
  if (sample && control && typeof sample.stack === 'string') {
    return [sample.stack, control.stack];
  }
}
```

What a component throws is mostly `new Error(..)`, of JS's own
constructor: a panic's form (ADR 0012). ADR 0660's `$unwound` made its
payload its message alone, `{ value: e.message, impl: $panicMessage }`, so
the port's `sample.downcast_ref::<&'static JsError>()` was `None` and the
`stack` was gone, and `resume_unwind` of it threw a new `Error` of its
message, whose `stack` is where it's resumed.

A panic's JS stays `throw new Error(message)`, which the user decided for
a port's `throw new Error` to read as React's (ADR 0660).

## Decision

**What unwound of a thrown `Error` of JS's own constructor, a panic's
`Error` whoever threw it, is its message and the `Error` both**: its
payload is `{ value: e.message, impl: $panicMessage, error: e }`, and
`payload.error`, `js::PANIC_ERROR`, is what JS threw.

- **A downcast to a `&str` or a `String` finds its message**, as ADR 0660's.
- **A downcast to a `&JsError` finds the `Error`**: what JS threw is the
  payload itself, or a panic's `Error`.

  ```rust
  if let Err(sample) = catch_unwind(render)
      && let Some(sample) = sample.downcast_ref::<&'static JsError>()
  {
      return sample.stack();
  }
  ```

  ```js
  const sample = $unwound(thrown);
  const error = sample.impl.typeId() === '&js::JsError' ? sample.value : sample.error;
  ```

  `is::<&JsError>()` is `.. || payload.error !== undefined`, and
  `Box::downcast` the `Ok` of either. A `downcast_mut` is the payload's
  own, as before.
- **`resume_unwind` of it throws the same `Error`** (`$unwinding`), its
  `stack` where it was made, as what JS threw is thrown again as itself;
  one whose message a `downcast_mut` changed, the panic of what it is now,
  as Rust resumes the `String` it changed.
- **Nothing else changes**: a `TypeError`, a `DOMException`, a string, is
  what JS threw alone; a payload `resume_unwind` was given is as it was;
  `$isPanic` and `$shouldPanic` are as they were.

## Why

- **React's `catch (sample)`** reads the error itself, its `stack`; and
  `try { .. } catch (x) { control = x; }` keeps it.
- **A panic stays `throw new Error(msg)`**: the payload, not the throw,
  says what it is, so no port's `throw new Error` reads otherwise.
- **The same object goes on**: a JS caller of a function that catches and
  resumes it catches what was thrown, as it would through JS's own frames.

## Alternatives

- **A thrown `Error` what JS threw alone, `{ value: e, impl: $jsThrown }`**:
  its message would be no `&str`, which a panic's is in Rust, and every
  `downcast_ref::<&str>()` of a panic would read it apart.
- **A downcast a dictionary call, `payload.impl.downcast(id)`**: every
  `dyn Any`'s downcast a call, where only a panic's payload is two things.
- **A brand on Rust's panics, `$Panic`**: the draft ADR 0660 replaced;
  React's `throw new Error` would read otherwise.

## Consequences

- **A panic's payload downcasts as a `&JsError` too**, Rust's own
  `panic!("boom")`'s, where Rust's finds none (case B, above): `from_js`
  of `lowering-unwinds`, whose `&JsError` arm is tried first, now reads a
  thrown `new Error("zero")` as `Error: zero`, not as the panic's text.
- **A panic resumed is the same `Error`**, where ADR 0660 made a new one of
  its message; its `stack` is where it was thrown. The corpus's
  `catch_unwind_payloads` and `catch_unwind_resumed`, against native
  Rust, print the same.

## What holds it

- `test/lowering-unwinds.test.ts`, "a thrown Error answers both
  downcasts": a JS `Error` caught, its `&str`, `String` and `&JsError`,
  its `stack`; `is` and `Box::downcast` of a panic's; `resume_unwind`
  throwing the same object; and "resume_unwind throws what unwound as it
  was", text resumed a panic.
- Corpus case `catch_unwind_message_changed` against native Rust: a
  payload resumed as it was, and changed first.
- Mutations `panic-error-lost`, `resumed-panic-made-again`,
  `resumed-panic-message-unchanged`
  (`runtime/unwind.js`), `js-error-downcast-of-panic-unseen`,
  `js-error-is-of-panic-unseen`, `js-error-box-downcast-of-panic-unseen`
  (`std_types/any.rs`).
