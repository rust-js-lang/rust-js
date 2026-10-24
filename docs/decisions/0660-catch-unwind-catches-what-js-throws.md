# 0660. `catch_unwind` catches what JS throws, a panic among it

Status: Accepted. Amends [0012](0012-panics-and-runtime-helpers.md), whose
panic Rust code couldn't catch; [0026](0026-testing.md), whose
`$shouldPanic` asks `$isPanic`; [0098](0098-destructors.md), whose drops
on a panic now run before a `catch_unwind` returns; [0331](0331-dyn-any.md),
whose `dyn Any` holds what unwound; and
[0637](0637-a-match-of-a-throwing-call-is-try-catch.md), whose `try` is
`catch_unwind`'s too, and a throwing call's `unwrap_or(d)` returned.
Replaces the draft "`catch_unwind` catches a Rust panic", which caught
panics alone, by the user's decision of 2026-10-23.
Amended by [0691](0691-a-dyn-any-drops-where-the-crate-makes-one-that-does.md):
a payload drops nothing where the crate makes no `dyn Any` of what drops,
so a crate with destructors catches as JS's `try` too, and a `match`
whose `Err` arm sets its binding again is `try` and `catch (thrown)`. Amended by
[0692](0692-a-thrown-error-is-a-panic-and-what-js-threw.md): a thrown
`Error` of JS's own constructor is what JS threw too, a `&JsError`, and
resumed is the same `Error`.
Amended by [0706](0706-panic-is-the-one-way-a-port-throws-an-error.md):
what a `catch` reads of what it caught is what was thrown, where it is,
`throw payload` of a payload resumed.
Amended by [0830](0830-what-follows-a-let-err-of-a-closure-that-never-returns-is-its-catch.md):
the `catch` of a `let Err(x)` of a closure that never returns holds what
follows that reads `x`, or what that declares.

Case ([0262](0262-when-rust-and-js-disagree.md)): N where a panic is
caught, as Rust catches it: the same closure runs once, the same drops
run as it unwinds, the same `Result` is given; B for what JS can't keep of
a payload and what native Rust aborts, each under Consequences; D where
the crate asked its panics to abort.

## Context

React's `getIODescription` describes any value by its getters, any of
which may throw:

```js
export function getIODescription(value) {
  try {
    switch (typeof value) { .. value.url .. value.toString() .. }
  } catch (x) {
    return '';
  }
}
```

A getter is a binding, `js::get(value, "url")`, whose throw is no `Err`:
JS throws through Rust's frames, as a panic does, a panic being `throw new
Error(message)` (ADR 0012). rust-js refused `std::panic::catch_unwind`, so
a port had no way to write React's `try`.

## Decision

**`std::panic::catch_unwind` is the one way Rust catches an unwind, and an
unwind is whatever JS throws through Rust's frames: a panic, or what a
binding, a getter or a callback threw.** `catch_unwind(f)` is `Ok` of
`f()`, or `Err` of what unwound, its payload, a `Box<dyn Any + Send>`:

```rust
pub fn get_io_description(value: Option<&Unknown>) -> String {
    catch_unwind(|| {
        let Some(value) = value else {
            return String::new();
        };
        match js::classify(value) { .. }
    })
    .unwrap_or_default()
}
```

```js
export function getIoDescription(value) {
  try {
    if (value == null) {
      return "";
    }
    ..
  } catch {
    return "";
  }
}
```

- **Its JS is JS's `try`, by ADR 0637's one rule.** Lowering makes
  `catch_unwind(f)` `$catchUnwind(f)` (`Expr::caught_unwind`), as it makes
  a throwing call `$try(() => call)`; preparation writes a `match` of
  either `Result`, its `if`, its `is_err()`, its `let _ =`, and now its
  `unwrap_or(d)` returned, `r.TAG === "Ok" ? r._0 : d`, as `try { .. }
  catch { .. }` (`Expr::caught`). What's tried is `f()`, a closure's body:
  `catch_unwind(|| describe(value))` is `try { return describe(value); }`,
  and a closure of statements whose value is returned is its body in the
  `try`, its `return`s the function's (`js::applied_tried`), where it
  never ends without one: a `switch` with a `default`, each of whose
  cases leaves, leaves, as `js::ends`, the one answer, now says; and so is
  one called as a statement whose body never returns, its names apart
  from the function's others as lowering names each (ADR 0352).
  Held or given, its `Result` is `$catchUnwind`'s; and so is one of a
  closure made by what may throw, `catch_unwind(made(n))`, as Rust makes
  it before it catches anything.
- **What unwound is `$unwound(e)`, a `dyn Any`'s pair (ADR 0331)**, which
  a `catch` that reads it makes of what it caught, `catch (thrown) { const
  payload = $unwound(thrown); .. }`:
  - **a panic's, its message**, `{ value: e.message, impl: $panicMessage
    }`, a `&'static str`, which `downcast_ref::<&str>()` finds; and
    `downcast::<String>()` and the rest find it too, `|| payload.impl ===
    $panicMessage`, since JS doesn't keep whether a panic's was a literal's
    `&str` or a formatted `String`;
  - **what JS threw, itself**, a `&'static JsError`, `{ value: e, impl:
    $jsThrown }`: `downcast_ref::<&JsError>()` gives the `TypeError`, its
    `name()` and `message()`;
  - **a payload `resume_unwind` threw, as it was**, `e.payload`.
- **A panic is told apart from what JS threw by one question,
  `$isPanic(e)`**: an `Error` of JS's own constructor, `e.constructor ===
  Error`, the form `panic!` throws (ADR 0012), whoever threw it, so a JS
  library's `throw new Error(..)` is a panic to Rust as React's port's
  `panic!` is a `throw new Error(..)` to JS; or a `$Unwinding`.
  `$shouldPanic` asks it too, and nothing else answers it.
- **`resume_unwind(payload)` is `throw $unwinding(payload)`**: what JS
  threw, itself, the same object a JS caller catches; a panic's message,
  or any text, the panic of it, `new Error(text)`; any other payload, a
  `$Unwinding` of it, an `Error` whose message is `Box<dyn Any>`, which a
  `catch_unwind` gives back as it was, `downcast::<i32>()` and all. No
  hook runs, as Rust's `resume_unwind` runs none.
- **The bounds are rustc's.** `F: FnOnce() -> R + UnwindSafe` is checked
  where it's written; an `AssertUnwindSafe` is the value it holds
  (`Recognition::transparent`), `catch_unwind(AssertUnwindSafe(|| count
  += 1))` its closure, and a closure with a destructor in one is where
  the call is, as one given to the call is.
- **Drops are Rust's.** A panic runs each scope's drops as it unwinds,
  each a `finally` (ADR 0098), so what the closure held, what it took by
  `move`, and what its callees held are dropped before `catch_unwind`
  returns; the payload in its `Err` is dropped where the `Err` is, through
  its dictionary's drop, once. `catch_unwind` of a closure holding values
  with destructors, and `resume_unwind` of a payload with one, are std
  calls that give what they're given on (`std_values`'s list).
- **A future unwinds where it's awaited.** `catch_unwind` of a closure
  that makes one, `|| async move { .. }`, is `Ok` of its promise; a panic
  in it rejects the promise, which `.await` throws, where Rust's future
  panics where it's polled, past the `catch_unwind` either way.
- **Where the crate asked its panics to abort, `-C panic=abort`, unwinding
  is refused**, `catch_unwind` and `resume_unwind`: rust-js's panics
  unwind whatever it's asked (ADR 0090), so a `catch_unwind` would catch
  what ends Rust's process. The driver keeps what was asked
  (`panics_abort`) before it makes panics unwind.
- **`set_hook` and `take_hook` stay refused.** rust-js has no hook: a
  panic is the host's error where nothing catches it (ADR 0012), and a
  caught one writes nothing, where Rust's default hook writes its message
  to stderr before it unwinds.

## Why

- **It's React's JS**, `try { .. } catch { return ''; }`, the closure's
  body in the `try`, with no helper.
- **One way to catch.** A JS throw through Rust's frames unwinds them as a
  panic does, its drops and all, so the one thing that catches an unwind
  catches both; a second way, a binding's `Result` for each getter, would
  make the port name each getter that may throw, which React's one `try`
  doesn't.
- **A panic stays `throw new Error(message)`**, which React's port writes
  where React does: marking it, `throw new $Panic(..)`, would have each
  port's `throw new Error` read otherwise.

## Alternatives

- **The draft's: catch only a branded panic, and let a JS throw through.**
  Every panic site would throw a branded error, React's `throw new Error`
  among them, and `getIODescription`'s getters would need a `Result`
  each. Replaced by the user's decision.
- **A panic's payload exactly `&str` or `String`, as Rust's**: which each
  is is known where it's thrown, a literal's or a formatted message, and
  only a mark at each site, `$panic\`..\``, would carry it to the `catch`.
  Rejected, as a brand is: a downcast that reads the message reads it
  either way.
- **A panic's payload a `String` alone**: `downcast_ref::<&str>()` of a
  literal panic's, which Rust finds, would find none.
- **`$catchUnwind(() => ..)` where it's written**: a helper and a closure
  where React writes `try`.

## Consequences

- **A panic's message downcasts as a `&str` and as a `String`**, where
  Rust's is one of them: a downcast that reads the message reads it
  either way; one that tells them apart, `downcast_ref::<String>()` of
  `panic!("boom")`'s, finds it where Rust finds none. Case B: rare, a
  program that tells its payload's type apart and not its text, and
  harmless, as what it reads is the message. Its `type_id()` is a
  `&str`'s. Every downcast of a `dyn Any` to a `String` reads the second
  test, `|| any.impl === $panicMessage`, which only a payload passes: the
  `dyn_any` corpus case's two.
- **What JS throws as `new Error(message)` is a panic**, its payload its
  message, not a `&JsError`: the `Error` is what a panic is. Any other
  throw, a `TypeError`, a `DOMException`, a string, is a `&JsError`.
- **A panic resumed is a new `Error` of its message**, its stack where
  it's resumed; what JS threw is resumed as itself.
- **A `drop` that panics while another panic unwinds replaces it, and
  `catch_unwind` catches the replacement**, where Rust aborts (ADR 0098's
  difference, which Rust code can now see). Case B: a destructor that
  panics as it unwinds, a program native Rust ends. Keeping Rust's would
  take a runtime that knows it's unwinding in each drop's `finally`, as
  the draft's `$aborting` proposed.
- **A caught panic writes nothing**, where Rust's default hook writes its
  message to stderr (`set_hook` is refused).
- The semantics page says it under panics, the limitations no longer list
  `catch_unwind`, and the exceptions page shows the `try`. A throwing
  call's `unwrap_or(d)` returned, `$try` and its `TAG` before, is `try {
  return f(); } catch { return d; }`: the `todomvc` snapshot's `load`, of
  `JSON.parse(json)`, and the guide's `configOrDefault`.

## What holds it

- `test/lowering-unwinds.test.ts`: `getIODescription`'s port, its `try` of
  the closure's body, run in Node with a getter that throws, a Proxy whose
  trap throws and a symbol; a panic's message as a `&str` and a `String`,
  what JS threw as a `&JsError`, a `catch (thrown)` beside an `Err` arm's
  `thrown`; `resume_unwind` of a panic, of a number, of what JS threw;
  `AssertUnwindSafe`; a closure's body beside the `Ok` arm, its names
  apart, and one that may end without a `return`; an async closure's
  promise; the refusal where panics abort.
- `test/lowering-bindings.test.ts`, "a returned unwrap_or of a throwing
  call is try and catch".
- Corpus cases against native Rust: `catch_unwind_payloads` (literal,
  formatted, index, `unwrap()`, parse, division, resumed, nested, a `&mut`
  through `AssertUnwindSafe`, `Box::downcast`), `catch_unwind_drops` (each
  scope's, a `move` closure's, a resumed payload's), `catch_unwind_resumed`
  (a caught panic resumed, uncaught).
- Mutations in `scripts/mutations/runtime/unwind.ts`,
  `scripts/mutations/lower/std_types/unwind.ts`, and in the lists of
  `any.rs`, `values.rs`, `wrappers.rs`, `drops/facts.rs`, `main.rs`,
  `recognition/calls.rs`, `prepare.rs`, `js/make.rs` and `js/applied.rs`.

## Amendment: a closure through `AssertUnwindSafe` is dropped

An `AssertUnwindSafe` is called as what it holds: its `call_once` calls
`(self.0)()`, then drops it as that returns or unwinds. A closure it holds
that's called by `&`, whose kind rustc infers from its body alone, as
`AssertUnwindSafe(..)` asks for no `Fn`, dropped nothing of what it took:
`catch_unwind(AssertUnwindSafe(move || { let _k = &kept; 7 }))` never
dropped `kept` (#196). `function_kind` answers for it as for the closure it
holds, so the call drops it as it returns, as std's does
(`catch_unwind_assert_drops`, `assert-unwind-safe-uncalled`). One called
by `FnOnce` alone drops what it took as its body ends, as before.
