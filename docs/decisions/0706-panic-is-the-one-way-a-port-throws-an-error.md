# 0706. `panic!` is the one way a port throws an `Error`, and what's caught is read as it was thrown

Status: Accepted, by the user's decision of 2026-10-24. Amends
[0012](0012-panics-and-runtime-helpers.md), whose panic of an empty
message was `new Error("")`, and [0660](0660-catch-unwind-catches-what-js-throws.md)
and [0692](0692-a-thrown-error-is-a-panic-and-what-js-threw.md), whose
`catch` read what it caught through `$unwound`, and whose `resume_unwind`
of it was `$unwinding(payload)`. Amended by
[0710](0710-every-unwinds-payload-is-the-js-error-thrown.md): a downcast to
a `&JsError` is what was thrown of any `try`, not only one of panics;
below: a `catch_unwind` that heads an `if let` chain is a `try`; one
tested after what an `if` asks first is a `try` inside it; and by
[0830](0830-what-follows-a-let-err-of-a-closure-that-never-returns-is-its-catch.md):
the `catch` of a `let Err(x)` is what follows that reads `x` or what that
declares, with no `let` before the `try`.

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
  that declares and the rest reads a `let` before the `try`. (Amended by
  0830: what follows that reads `x` or what that declares, transitively,
  with no `let` before the `try`.)

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
  through a temporary, `catch (x) { const value = x; control = value; }`
  (0830): a capture's assignment isn't read in place yet.

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

## Amendment: a `catch_unwind` that heads an `if let` chain is a `try`

React's `DetermineComponentFrameRoot` catches its sample, then tests the
control its `try` set:

```js
try { .. } catch (sample) {
  // This is inlined manually because closure doesn't do it for us.
  if (sample && control && typeof sample.stack === 'string') {
    return [sample.stack, control.stack];
  }
}
return [null, null];
```

Its port writes `if let Err(sample) = catch_unwind(AssertUnwindSafe(|| {
.. })) && let Some(sample) = sample.downcast_ref::<&'static JsError>() &&
let Some(control) = control && let Some(sample_stack) = sample.stack() {
return ..; }`, and rust-js wrote `const value = $catchUnwind(() => {..});
if (value.TAG === "Err" && control) { let sample = value._0; sample =
sample.error; .. }`: the downcast is always `Some` (0710), so its test
left the chain, and the `if`'s test was the `Result`'s `TAG` and the
rest, which `try_catch` didn't know.

**An `if let Err(e) = catch_unwind(f) && c` chain is `try { f's body }
catch (e) { if (c) { .. } }`**: the `if` of its `Err` alone, `c`'s inside
it (`unchained`, `prepare/catches.rs`), which is then the `match` of a
`Result` this ADR and 0660 make a `try`.

- **`c` is tested in the `catch`**, once the body threw, as Rust tests it
  once `catch_unwind` returned: what the closure set, `control`, is what
  `c` reads, the closure's body in the `try`.
- **What the arm sets the payload's binding to first, `e = e.error;`, is
  before `c`**, which reads neither, so the `catch` is of what was thrown,
  `catch (sample)`; `c`'s `if` is where `c` is written, after a comment
  before it.
- **An `else`, where the arm leaves, is after the `try`**, as it's what
  runs where the arm doesn't: `try { f(); } catch { if (wanted) { return
  "caught"; } } return "not";`. One where the arm goes on keeps
  `$catchUnwind`, as its `else` runs where `c` is false and not where the
  arm ran.
- **A `Result` tested where it's made**, `if ($catchUnwind(f).TAG ===
  "Err" && c)`, is one made, then tested (`made_where_tested`).
- **A closure's body whose value nothing reads, `(() => { ..; return
  step(n); })();`, is its body**, `..; step(n);`, the `return`'s value a
  statement where it does something (`js::applied_tried`).
- A chain an `Ok` heads, and one whose `Ok` is read after it, keep
  `$catchUnwind`, as before.

Case ([0262](0262-when-rust-and-js-disagree.md)): A, the same tests in
the same order. Held by `test/lowering-unwinds.test.ts`, "a catch_unwind
that heads an if let chain is a try", run in Node; corpus case
`catch_unwind_chained` against native Rust; and mutations
`chain-not-unchained`, `chain-tested-where-made-kept`,
`chain-else-of-going-on-after`, `chain-else-dropped`,
`chain-shadow-left-in-if`, `chain-if-before-its-comment`
(`prepare/catches.rs`), `tried-last-return-dropped` (`js/applied.rs`).

## Amendment: a `catch_unwind` tested after what an `if` asks first is a `try` inside it

React's checkPassiveEvents asks whether it has a DOM, then tries:

```js
if (canUseDOM) {
  try {
    const options = {};
    Object.defineProperty(options, 'passive', {get: function () { passiveBrowserEventsSupported = true; }});
    window.addEventListener('test', options, options);
    window.removeEventListener('test', options, options);
  } catch (e) {
    passiveBrowserEventsSupported = false;
  }
}
```

Its port writes clippy's form, `if can_use_dom &&
catch_unwind(AssertUnwindSafe(|| { .. })).is_err() { .. }` (clippy's
`collapsible_if` asks for it over an `if` in an `if`), and rust-js wrote
`if (canUseDOM && $catchUnwind(() => { .. }).TAG === "Err") { .. }`
(rd222): `made_where_tested` took a `Result`'s test only at the chain's
head.

**`if (c && $catchUnwind(f).TAG === "Err" && d) { .. }`, with no `else`,
is `if (c) { try { .. } catch { if (d) { .. } } }`** (`asked_first`,
`prepare/catches.rs`): what's before the `Result`'s test is the outer
`if`'s, and the rest is the `if` this ADR already makes a `try`, inside
it. A `$try` of a JS call (ADR 0637) is split the same way.

- **It's the same program**: `if (a && b) S` with no `else` is `if (a) {
  if (b) S }` in JS, `b` run only where `a` is true, as `&&` runs it, so
  the closure runs only where `c` is true, as Rust's `&&` calls
  `catch_unwind` only then.
- **Only where the rest is a `try`**: where it isn't (an `Ok` read after
  it), the `if` stays one test, as before, not two `if`s around
  `$catchUnwind`.
- **An `else` keeps `$catchUnwind`**: it runs both where `c` is false and
  where the closure didn't throw, so splitting the test would write it
  twice.

Case ([0262](0262-when-rust-and-js-disagree.md)): none, the same tests in
the same order. Held by corpus case `catch_unwind_tested_last` against
native Rust (one asked first, two, one after, an `Ok`, an `else`);
`test/lowering-unwind-arms.test.ts`, "a JS call's Result tested after what
an if asks first is a try inside it", run in Node; the guide's `passive` (`interop/exceptions.md`); and mutations
`asked-first-kept-in-test`, `asked-first-only-the-first`,
`asked-first-else-dropped` (`prepare/catches.rs`).
