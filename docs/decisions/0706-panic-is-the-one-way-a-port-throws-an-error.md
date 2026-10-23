# 0706. `panic!` is the one way a port throws an `Error`, and what's caught is read as it was thrown

Status: Accepted, by the user's decision of 2026-10-24. Amends
[0012](0012-panics-and-runtime-helpers.md), whose panic of an empty
message was `new Error("")`, and [0660](0660-catch-unwind-catches-what-js-throws.md)
and [0692](0692-a-thrown-error-is-a-panic-and-what-js-threw.md), whose
`catch` read what it caught through `$unwound`, and whose `resume_unwind`
of it was `$unwinding(payload)`. Amended by
[0710](0710-every-unwinds-payload-is-the-js-error-thrown.md): a downcast to
a `&JsError` is what was thrown of any `try`, not only one of panics.

Case ([0262](0262-when-rust-and-js-disagree.md)): A where what's read is
the same value, `$unwinding($unwound(e))` being `e`; for `new Error()`,
what the program can't observe: its `message` is `""`, as `new
Error("")`'s, which JS keeps as its own property and `new Error()` as its
prototype's, and Rust reads only the text.

## Context

React's `DetermineComponentFrameRoot` makes its control error by throwing
one, so the engine writes its `stack` where it's thrown:

```js
try {
  throw Error();
} catch (x) {
  control = x;
}
```

A port had three ways to write it, none reading as React's:
`Some(js::js_error::new(""))`, `new Error('')` made, not thrown;
`resume_unwind(Box::new(error))` or `panic_any(error)`, which rustc
refuses, as a `&JsError` isn't `Send`; and `catch_unwind(|| panic!(""))`,
whose JS was `throw new Error("")` and a `catch (thrown) { const x =
$unwound(thrown); control = x.impl.typeId() === "&js::JsError" ? x.value :
x.error; }`.

## Decision

**`panic!` is the one way a port throws a JS `Error`.** `panic!("msg")` is
`throw new Error("msg")`, as before, and `panic!("")` is `throw new
Error()`: `Expr::panic`, the one place rust-js makes a panic's `Error`.
`resume_unwind` and `panic_any` of a `&JsError` stay rustc's refusal.

**What a `catch` reads of what it caught is what was thrown**, where that
is what it reads:

- **`resume_unwind(payload)` of a payload only read**, its fields and
  nothing written through it, is `throw thrown`: the same object, as
  `$unwinding($unwound(e))` is `e`.
- **A downcast to a `&JsError` where the `try` is only panics**, `throw
  new Error(..)` of literals and variables, is the `Error` thrown: an
  operator or a read in it may run JS that throws any value, `"" +
  value` a `TypeError` of a symbol.
- **A `catch` that reads the payload no other way is of it**, by the
  `Err`'s binding: `catch (x)`.
- **`let Err(x) = catch_unwind(f)` of an `f` that never returns**, whose
  `Ok` is none, `!`'s, is the `try` too: its `catch` what follows, up to
  its last read of `x`, as what follows runs only once `f` threw; what
  that declares and the rest reads a `let` before the `try`.

```rust
let Err(x) = catch_unwind(|| panic!(""));
control = x.downcast_ref::<&'static JsError>().copied();
```

```js
try {
  throw new Error();
} catch (x) {
  control = x;
}
```

```rust
match catch_unwind(render) {
    Ok(n) => n,
    Err(payload) => resume_unwind(payload),
}
```

```js
try {
  return render();
} catch (payload) {
  throw payload;
}
```

## Why

- **It's React's JS**: the `throw`, the `catch (x)` and `control = x`.
- **One way**: a port writes a throw as Rust writes one, a panic; the
  `Error` it makes is the panic's, a `&JsError` to a `catch_unwind`.
- **The same object**: Rust's `resume_unwind` gives on the box it was
  given, and JS's `throw` of what was caught the object it caught; a
  payload of the program's own is the same `$Unwinding`, where
  `$unwinding` made another of it.

## Alternatives

- **`js::throw(error) -> !`**, a binding for JS's `throw`: a second way to
  unwind beside a panic, and the user decided against it.
- **`panic_any(error)` of a `&JsError`**: rustc's `Send` bound refuses it,
  as it should; a payload that crosses threads in Rust isn't JS's.

## Consequences

- `panic!("")` reads `new Error()` everywhere: in a `catch_unwind`, a
  `Drop`, a function's body.
- A `catch` whose payload is read otherwise, `downcast_ref::<String>()`,
  keeps its `$unwound(thrown)`; one whose `try` calls what may throw
  anything keeps its downcast's test, as what's thrown may be any value.

- A `let Err(x)` in a closure that sets a variable it captures reads
  through a temporary, `let value; try { .. } catch (x) { value = x; }
  control = value;`: a capture's assignment isn't read in place yet.

## What holds it

- `test/lowering-unwinds.test.ts`, "a port throws an Error by panic! and
  catches it as thrown", run in Node: the control error's `stack`, a
  resume rethrowing the same object, and a payload downcast to `String`.
- Corpus case `catch_unwind_rethrown` against native Rust; the
  architecture test "a panic's Error is made in one place".
- Mutations `resumed-made-again`,
  `resumed-after-write`, `thrown-error-of-any-try` (replaced by 0710's
  `thrown-error-read-as-payload`),
  `catch-unnamed`, `unwound-let-kept`, `unwound-let-declared-inside`
  (`prepare/catches.rs`), `panic-of-nothing-written` (`js/make.rs`).
