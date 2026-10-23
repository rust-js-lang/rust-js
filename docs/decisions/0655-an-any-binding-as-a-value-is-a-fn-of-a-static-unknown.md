# 0655. A binding of `any` as a value is a `fn` of `Option<&'static Unknown>`

Status: Accepted. Extends [0602](0602-a-binding-taken-as-a-fn-is-its-function.md),
whose binding taken as a `fn` is its JS function, and
[0601](0601-a-module-variable-an-if-makes-is-a-let.md).

Case ([0262](0262-when-rust-and-js-disagree.md)): N: the function is
JS's own, read where it's taken.

## Context

React's `reportGlobalError` is the host's `reportError` where it has
one, and a polyfill where it hasn't:

```js
const reportGlobalError: (error: mixed) => void =
  typeof reportError === 'function' ? reportError : error => { .. };
```

webapi binds `reportError(e: any)` as a generic, `report_error<E>(e: E)`,
as each `any` parameter is (ADR 0225). Its port declared the value
`fn(Option<&Unknown>)`, and rustc refused `webapi::global::report_error`
there, "one type is more general than the other": that `fn` takes an
`Option<&'a Unknown>` of every `'a`, and a generic's instance is of one.

## Decision

**A binding of `any` taken as a value is a `fn` of `Option<&'static
Unknown>`, a JS value of any shape, which lives as long as JS keeps it,
as every binding gives one, `&'static Unknown`.** Its generic is then
`Option<&'static Unknown>`, one type, and the binding coerces:

```rust
thread_local! {
    static REPORT_GLOBAL_ERROR: fn(Option<&'static Unknown>) =
        if js::is_defined(webapi::global::report_error::<Option<&Unknown>>) {
            webapi::global::report_error
        } else {
            |error| { .. }
        };
}
js::export_default!(REPORT_GLOBAL_ERROR);
```

```js
let reportGlobalError;
if (typeof reportError !== "undefined") {
  reportGlobalError = reportError;
} else {
  reportGlobalError = (error) => { .. };
}

export default reportGlobalError;
```

- **The binding is its function**, `reportError`, read where it's taken
  (ADR 0602); a value of it alone, `static HOST: fn(..) =
  webapi::global::report_error`, is the module's `const`, `const
  hostReportError = reportError`.
- **`is_defined` names the instance**, `report_error::<Option<&Unknown>>`,
  as a generic function's value needs one; the lifetime there is any.
- **No compiler change**: the form is rustc's to check. rustc's own error
  says what's wrong of the other one.

## How close it is to React's

- **`typeof reportError !== "undefined"`**, where React tests `===
  'function'`: whether the host has a global is `js::is_defined`'s one form
  (ADR 0597).
- **`let` and an `if` that sets it**, where React writes one `const` of a
  `?:`: a thread-local an `if` makes is a `let` each branch sets (ADR
  0601).
- **A comment before a branch's value is lost**, `// In modern browsers,
  reportError will dispatch an error event`: a comment before a block's
  tail expression is, here and in any function, `if flag { // c\n 1 }`.
  A gap of its own, not this decision's.
- `error` of the polyfill's `ErrorEvent` is `error: Some(&error)` (ADR
  0654), and Node's `process.emit('uncaughtException', error)` is
  `process::emit(UncaughtException, (error,))` (ADR 0656).

## Why

- **A JS value's type is a `'static` borrow** wherever a binding gives
  one, so a function of one is what a JS caller can give it.
- **One type for the binding and its fallback**: the closure is given
  what the binding is, `Option<&'static Unknown>`, and inspects it as any
  `Unknown`.

## Alternatives

- **`any` parameters of `Option<&Unknown>`, not generic**: a function of
  every lifetime, but a Rust value, a struct, would need a cast to be
  given to `reportError`, as `postMessage` and each `any` would.
- **`&dyn IntoUnknown` parameters**: of every lifetime too, but the
  fallback could do nothing with what it's given.
- **A closure that calls the binding**, `|error| report_error(error)`:
  `(error) => reportError(error)`, which reads `reportError` at each call,
  where React captures it once.

## What holds it

- `test/bindings.test.ts`, "a binding of any as a value is a fn of a
  static Unknown": the host's `reportError` taken where it's defined, the
  fallback where it isn't, and a value of it alone a `const`, each run.
- The guide's [binding as a value](../guide/interop/bindings.md#a-binding-as-a-value).
