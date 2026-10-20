# `catch_unwind` is a `try`

Status: Draft. Would extend [0012](../0012-panics-and-runtime-helpers.md),
whose panic is a thrown `Error`, and [0098](../0098-destructors.md), whose
scope holding a value with a destructor is a `try`.

Case: C ([0262](../0262-when-rust-and-js-disagree.md)) for what's caught
and what the result is; B for the payload's type, listed below.

## Context

A panic is a thrown `Error` (ADR 0012), so every scope that must drop
unwinds through `finally` already. `catch_unwind` is refused, though it's
the one std API whose JS is the construct it's named after, and the
`should_panic` runtime already catches a panic to test it.

## Decision

**`std::panic::catch_unwind(f)` is `$catchUnwind(f)`: a `try` that calls
`f` and gives `Ok(result)`, or catches a panic and gives `Err(payload)`,
the panic's message as a `Box<dyn Any>` that `downcast_ref::<&str>` and
`::<String>` read. What isn't a panic, a JS error of the host's, isn't
caught: it's rethrown, as `should_panic` rethrows it.**

```js
const r = $catchUnwind(() => risky(n));
```

- **`resume_unwind(payload)`** rethrows the same `Error`.
- **`AssertUnwindSafe`** is its value, a wrapper rustc checks and JS
  needs nothing of.
- **`panic::set_hook` stays refused**: a hook runs before unwinding, and
  rust-js's panic has no place for one yet; `take_hook` with it.
- **A payload that isn't a string**, `panic_any(5)`, is refused at
  `panic_any`: a panic's `Error` carries its message alone.

## Why

- **It's std's**: a corpus case catches a panic, reads its message both
  ways, resumes one, and runs a destructor in the function that panicked,
  checked against native Rust; a test holds a host's error passes through.
- **It's the JS a person writes**: a `try`.

## Consequences

- **The limitations page loses `catch_unwind`**, and names `panic_any` of
  a non-string payload under Panics.
- **One listed difference** (B): the payload is the message, where Rust's
  may be any value.
