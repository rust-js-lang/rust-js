# A `&JsError` downcast of a caught unwind is what was thrown

Status: Draft, waiting on the user's decision below. Would amend
[0692](../0692-a-thrown-error-is-a-panic-and-what-js-threw.md) and
[0706](../0706-panic-is-the-one-way-a-port-throws-an-error.md).

## Context

React's `getOwnerStackByComponentInfoInDev` (shared/ReactComponentInfoStack.js)
and `DetermineComponentFrameRoot` (shared/ReactComponentStackFrame.js) read
what they caught: `catch (x) { .. x.message + '\n' + x.stack }`. Their port
writes `match x.downcast_ref::<&'static JsError>() { Some(e) => .. e.stack()
.., None => .. }`, and rust-js writes

```js
} catch (thrown) {
  const x = $unwound(thrown);
  const x$1 = x.impl.typeId() === "&js::JsError" ? x.value : x.error;
  if (x$1) { .. x$1.stack .. }
  return `..${(x.impl.typeId() === "alloc::string::String" || x.impl === $panicMessage ? x.value : undefined) ?? ""}..`;
}
```

ADR 0706 reads the downcast as `thrown` only where the `try` throws
nothing but its own panics, `throw new Error(..)` of literals.

## When the downcast is `Some(thrown)`

`$unwound` of what was thrown gives (`runtime/unwind.js`):

| Thrown | Payload | `downcast_ref::<&JsError>()` |
|---|---|---|
| an `Error` of JS's own constructor, a panic's included | `{ value: message, impl: $panicMessage, error: e }` | `Some(e)`, ADR 0692 |
| any other JS value, a `TypeError`, a string | `{ value: e, impl: $jsThrown }` | `Some(e)` |
| a `$Unwinding`, `panic_any(v)` or `resume_unwind(Box::new(v))` of a `v` that's not text | `e.payload`, of `v`'s type | `None`, as Rust's |

So **the downcast is `Some(thrown)` exactly where what's thrown is no
`$Unwinding`**: where no code the `try` runs makes a payload of another
type than text. (A `&JsError` can't be one: it isn't `Send`.) And its
`None` arm runs today for a thrown `undefined`, `null`, `0` or `""` too,
as an `Option<&JsError>` is tested by truthiness; read as `thrown`, those
would run the `Some` arm with a value no `&JsError` is.

## The gate: what shows no `$Unwinding` reaches the `catch`

Neither React `try` can be shown so by what rust-js sees when it compiles
the crate:

- `DetermineComponentFrameRoot` calls `r#fn`, a `&dyn Fn`, a component
  from anywhere.
- `getOwnerStackByComponentInfoInDev` reads `error.stack`, a getter that
  runs `Error.prepareStackTrace`, any function, a Rust one of another
  crate too.

A crate's dependents compile after it, and a JS app may load crates built
apart, so no crate-wide or library-contract fact says no payload of
another type is made. The options, each for the user to decide:

1. **A payload of another type downcasts as a `&JsError` too, the
   `$Unwinding` thrown** (ADR 0262 case B, as 0692's panic text is): every
   `&JsError` downcast is `thrown`, and both files read as upstream's. A
   program that tries `&JsError` before its own payload type would take
   the first arm where Rust takes the second.
2. **`panic_any` and `resume_unwind` of a payload that's not text, nor a
   caught one, refused**: no `$Unwinding` is ever thrown, and the
   downcast is `thrown` everywhere. A refusal where Rust is fine, and
   rustc's tests of typed payloads would fail.
3. **Keep the test where the `try` may run code rust-js doesn't see**, as
   now; generalize 0706 only to a `try` of this crate's code that makes
   no typed payload and reads no JS. Neither React file changes.

What would hold it: `test/lowering-unwinds.test.ts`, the InfoStack shape
run with a `TypeError`, a panic, and (by the choice) a `panic_any(5)`;
the corpus case `catch_unwind_payloads` against native Rust.
