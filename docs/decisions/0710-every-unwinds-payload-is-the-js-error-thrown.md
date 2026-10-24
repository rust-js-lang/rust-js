# 0710. Every unwind's payload downcasts as the `&JsError` thrown

Status: Accepted, by the user's decision of 2026-10-24. Amends
[0692](0692-a-thrown-error-is-a-panic-and-what-js-threw.md), whose
`&JsError` of a payload was what JS threw or a panic's `Error` alone, and
[0706](0706-panic-is-the-one-way-a-port-throws-an-error.md), whose
`catch` read its `&JsError` as what was thrown only where the `try`
threw nothing but its own panics. Amended below: what's given its
`Some`'s value, `and_then`'s closure, is given it, `x.stack`.

Case ([0262](0262-when-rust-and-js-disagree.md)): B, a payload of
another type than text, `resume_unwind(Box::new(5))`'s, that downcasts as
a `&JsError` too, where Rust's is `None`: rare, a program that tries
`&JsError` before its own payload's type, and harmless, as what it finds
is the `Error` the payload was thrown in, and the payload still downcasts
to its type. A falsy throw, `throw undefined`, has no Rust counterpart:
what was thrown, a `Some` where its `Option` is tested.

## Context

React's `getOwnerStackByComponentInfoInDev` (shared/ReactComponentInfoStack.js)
and `DetermineComponentFrameRoot` (shared/ReactComponentStackFrame.js) read
what they caught: `catch (x) { return '\nError generating stack: ' +
x.message + '\n' + x.stack; }`. Their port writes `match
x.downcast_ref::<&'static JsError>() { Some(x) => .. x.stack() .., None =>
.. }`, and rust-js wrote

```js
} catch (thrown) {
  const x = $unwound(thrown);
  const x$1 = x.impl.typeId() === "&js::JsError" ? x.value : x.error;
  if (x$1) { .. x$1.stack .. }
  return `..${(x.impl.typeId() === "alloc::string::String" || x.impl === $panicMessage ? x.value : undefined) ?? ""}..`;
}
```

The downcast was what was thrown wherever what was thrown was no
`$Unwinding`, the `Error` a payload of another type is thrown in; and its
`None` arm ran for a thrown `undefined`, `null`, `0` or `""`, as an
`Option<&JsError>` is tested by truthiness. Neither React `try` can be
shown to make no such payload: one calls a component, the other reads
`error.stack`, which runs `Error.prepareStackTrace`.

## Decision

**Every unwind's payload downcasts as a `&'static JsError`, the JS value
that was thrown**: a panic's `Error`, what JS threw, or the `$Unwinding` a
typed payload, `resume_unwind(Box::new(v))`'s, is thrown in. Its
`downcast_ref::<&JsError>()` is always `Some`, a falsy throw's too.

- **The payload holds it, `payload.error`** (`js::THROWN_VALUE`): `$unwound`
  of what JS threw is `{ value: e, impl: $jsThrown, error: e }`, and a
  `$Unwinding` is its payload's `error`, so `resume_unwind` of it throws the
  same `$Unwinding` again.
- **A typed payload still downcasts to its own type**, as native Rust's:
  `downcast_ref::<i32>()`, `is`, `downcast_mut`, `Box::downcast`.
- **A downcast to a `&JsError` of a `dyn Any + Send` is `payload.error`**
  (`FnCx::finds_thrown`): a `dyn Any + Send` holds no `&JsError` of its
  own, as one isn't `Send`, so it's an unwind's payload. `is` of it is
  `true`, `Box::downcast` its `Ok`. A `dyn Any` without `Send`, a payload
  widened or a box of the program's own, keeps 0692's test.
- **Its `Some` is known where it's tested**: a `match`, an `if let`, a
  `let else`, a `?` of the downcast's variable, written once, is its
  `Some` (`OptionWrites::thrown`), so the `None` arm is dead and left out;
  rustc still checks it.
- **A `catch` reads `payload.error` as `thrown`**, where 0706 did so for
  a `try` of panics alone, and a payload read no other way is the
  `catch`'s binding: `catch (x)`. A shadow its arm sets to it first, `let
  x = $unwound(thrown); x = x.error;`, is the binding too.

```rust
match catch_unwind(render) {
    Ok(info) => info,
    Err(x) => match x.downcast_ref::<&'static JsError>() {
        Some(x) => format!("\nError generating stack: {}\n{}", x.message(), x.stack().unwrap_or_default()),
        None => format!("\nError generating stack: {}\n", x.downcast_ref::<String>().cloned().unwrap_or_default()),
    },
}
```

```js
try {
  return render();
} catch (x) {
  return `\nError generating stack: ${x.message}\n${x.stack ?? ""}`;
}
```

## Why

- **It's React's JS**: `catch (x)`, `x.message`, `x.stack`, with no
  `$unwound` and no test of what was thrown; and the stack frame's `catch
  (sample)` and `catch (x) { control = x; }`.
- **One answer**: what a `&JsError` downcast of a payload is no longer
  depends on what the `try` calls, a component or a getter, which rust-js
  can't see when it compiles the crate.
- **The program's own types keep Rust's**: a payload of a type is still
  that type; only the `&JsError` it can't hold natively is added.

## Alternatives

- **`panic_any` and `resume_unwind` of a payload that's not text
  refused**: no `$Unwinding` thrown, so the downcast is what was thrown
  everywhere; a refusal where Rust is fine, and rustc's tests of typed
  payloads would fail.
- **The test kept where the `try` may run code rust-js doesn't see**, as
  0706 had it: neither React file changes.
- **Falsy throws `None`**, as truthiness reads them: the `None` arm stays
  live in every port's `catch`, for values no `&JsError` is.

## Consequences

- **A typed payload's `downcast_ref::<&JsError>()` is `Some` of its
  `$Unwinding`**, `None` natively (case B, above); `is::<&JsError>()` is
  `true`, and `Box::downcast::<&JsError>()` `Ok`.
- **A `Box<dyn Any + Send>` the program makes and never throws** downcasts
  as a `&JsError` too, `undefined`, where Rust's is `None`: one can't hold
  a `&JsError`, so nothing has a reason to ask.
- **A falsy throw's `Option` carried on as a value**, `.copied()`,
  `.is_some()`, `.unwrap()`, is `None` (given to a closure, it's given
  it: the amendment below), as an `Option<&JsError>`'s `None`
  is `undefined` and JS's `if (x)` reads it so; only where it's tested is
  it known to be `Some`.
- `panic_any` stays refused, as before; `resume_unwind(Box::new(v))` is
  the typed payload a program writes.

## What holds it

- `test/lowering-unwinds.test.ts`, "every unwind's payload downcasts as the
  JsError thrown", run in Node: React's InfoStack `catch (x)` and its
  stack frame's `catch (x) { control = x; }`, a `TypeError`, a panic,
  `undefined`, `null`, `0`, `""`, `false` and `5` thrown, a typed
  payload's `$Unwinding`, its `is` and `Box::downcast`, `?` of it, one set
  again, a payload passed on, one widened to a `dyn Any`, a `dyn Any` of
  the program's own `&JsError`, and a typed payload resumed as the same
  `$Unwinding`.
- Corpus case `catch_unwind_typed_payloads` against native Rust: typed
  payloads downcast, changed, resumed and taken out as Rust's.
- Mutations `thrown-of-any-send-unseen`, `thrown-is-false`,
  `thrown-box-downcast-err`, `thrown-of-any-dyn` (`std_types/any.rs`),
  `thrown-written-again`, `thrown-tried-breaks` (`mir/patterns.rs`),
  `thrown-tested-none` (`mir/rvalues.rs`), `js-throw-error-lost`,
  `unwinding-error-unset`, `resumed-unwinding-made-again`
  (`runtime/unwind.js`), `thrown-error-read-as-payload`,
  `thrown-shadow-unwound` (`prepare/catches.rs`),
  `thrown-error-unrecognized` (`js/make.rs`), `dyn-any-send-of-any`
  (`recognition/types.rs`).

## Amendment: what's given its `Some`'s value is given it

React's `describeBuiltInComponentFrame` reads the stack of what it threw,
`catch (x) { const match = x.stack.trim().match(..); }`. Its port writes
`x.downcast_ref::<&'static JsError>().and_then(|x| x.stack())`, and
rust-js wrote `x?.stack`: the downcast's `Some` was known where it was
tested, a `match`, an `if let`, a `?`, but `and_then` tested it again.

**What's given an unwind's payload's `&JsError`, the closure of
`and_then`, `map` or `is_some_and` of its downcast, is given it**, as Rust
gives a `Some`'s value to it (`mir_thrown_given`, `mir/dispatch.rs`, of
a local `OptionWrites::thrown` says): `and_then(|x| x.stack())` is
`x.stack`,
`map(|e| e.message())` `x.message`, `is_some_and(p)` `p(x)`; `map`'s
value boxed where it looks like `None`, `map(|_| None)`'s `Some(None)`
(ADR 0051). `.copied()` of it was `x` already, as it gives what it's
given.

Case ([0262](0262-when-rust-and-js-disagree.md)): A for every payload
Rust has, whose `&JsError` is always `Some`; a falsy throw, which has no
Rust counterpart, is given to the closure too, so `throw null`'s
`x.stack` throws a `TypeError` from the `catch`, as React's does, where
`x?.stack` was `None`.

Held by `test/lowering-unwinds.test.ts`, "what's given the thrown
downcast is given it as it is", run in Node, and mutations
`thrown-given-tested` and `thrown-mapped-unboxed` (`mir/dispatch.rs`).
