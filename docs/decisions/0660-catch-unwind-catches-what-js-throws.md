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
follows that reads `x`, or what that declares. Amended below: an `Ok`
arm that tests what it holds is in the `try`, and a binding's setter is a
write that may throw; and the `try`'s value is named by the pattern that
binds all of it. Amended by
[0776](0776-get-has-set-take-any-value.md) (#462): what's tried of a
closure of one `return v;` is the closure called, which tidy's applied
arrows make `v` with the closure's place, and a `v` nothing reads that
may throw, a JS operator's of any value, stays.

Case ([0262](0262-when-rust-and-js-disagree.md)): N where a panic is
caught, as Rust catches it: the same closure runs once, the same drops
run as it unwinds, the same `Result` is given; B for what JS can't keep of
a payload, under Consequences; R for a drop's panic as a panic unwinds,
which aborts as Rust's does (amended below, #847); D where the crate
asked its panics to abort.

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
- **A `drop` that panics while another panic unwinds aborts the
  process**, as Rust does: amended below (#847). It replaced the panic,
  and `catch_unwind` caught the replacement, until the user's decision of
  2026-11-06.
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

## Amendment: an `Ok` arm that tests what it holds, and a setter's write

Case ([0262](0262-when-rust-and-js-disagree.md)): N for the first, the
same code run in the same order; R for the second, a wrong answer fixed.

React's `startGestureTransition` returns what the renderer gives, where
it gives one, and reports what its scope threw:

```rust
match panic::catch_unwind(AssertUnwindSafe(|| {
    ..
    g(REACT_SHARED_INTERNALS).map(|finish| finish(&current_transition, provider, options))
})) {
    Ok(Some(finish)) => return finish,
    Ok(None) => {}
    Err(error) => REPORT_GLOBAL_ERROR.with(|report_global_error| report_global_error(..)),
}
```

It was `const value = $catchUnwind(() => { .. }); if (value.TAG ===
"Ok") { if (value._0) { .. return finish; } } else { .. }`, where upstream
is a `try` that returns inside and a `catch`. The `Ok` arm had no binding
of its own, `const v = result._0;`, as a pattern inside it tests the value
first, so the `try` wasn't made.

**An `Ok` arm that reads its value only as `result._0` is the value's, by
the result's own name**: `try { ..; const value = finish ? finish() :
undefined; if (value) { const finish = value; return finish; } } catch
(error) { .. }`, the closure's body in the `try` and its value the
`const` (`js::applied_tried`), what it declares named apart from the arm's
as lowering names each. The arm is in the `try` only where it can't throw,
as before, an `if` of a test that can't, of what can't, among it.

**A binding's setter is a write that may throw** (`Known::Getter` on its
property, as a getter's read is, ADR 0672): `INTERNALS.T = v` runs what
JS defines, which may throw. It was a write that only makes a value, so
an `Ok` arm that set one, `Ok(v) => { set_t(INTERNALS, v); v }`, was in
the `try`, `try { const v = scope(); INTERNALS.T = v; return v; } catch
{ return 0; }`, and its throw was caught, where Rust's panic out of the
arm goes on. Now the arm runs after the `try`, `let v; try { v = scope();
} catch { return 0; } INTERNALS.T = v; return v;`.
The same write in a scope that drops something, a borrow's or a guard's,
is in the `try` whose `catch` drops it, as any statement that may throw
is: `view.list.textContent = ""` of the todo example's render, which ran
before its `try`, so a throw from it left its `RefCell` borrowed.

Held by `test/corpus/catch_unwind_nested_ok.rs`, against native Rust (a
guard's drop after an `Ok(Some(..))` that returns, through a panic too;
an arm with a guard; one that panics itself, which nothing there catches);
`test/lowering-unwinds.test.ts`, "an Ok arm that sets a binding's property
runs after catch_unwind's try", whose setter throws; and the mutations
`ok-value-read-in-place-unseen`, `ok-value-reads-counted-as-results`,
`cannot-throw-if-arms-unread`, `cannot-throw-if-refused`
(`prepare/catches.ts`), `tried-const-called` (`js/applied.ts`) and
`setter-write-only-makes-a-value` (`lower/binding_calls.ts`).

## Amendment (2026-10-25): the `try`'s value by its pattern's name

Case ([0262](0262-when-rust-and-js-disagree.md)): N, the same value read
in the same places; only its name changes.

The amendment above gave the `try`'s value the result's name and kept the
pattern's binding of it, `const value = finish ? finish() : undefined; if
(value) { const finish = value; return finish; }`: two names of one
value, where upstream has none but the test's.

**Where the pattern binds the whole value, `Ok(Some(f))`'s `const f =
value;`, the value is `f`, its binding gone**: `const finish = g ? g(..) :
undefined; if (finish) { return finish; }`, as an `if let Some(v) =
o.map(..)` outside a `catch_unwind` reads. Only where that's its one
name: the binding declared once in the arm, read only after it there, and
neither its name nor one given apart from it, `finish$1`, read or
declared in the closure, whose body the `try` runs first. Else the value
keeps the result's name, as before, so neither is given apart to be the
other: `const picked = pick(n); const value = picked != null ? .. :
undefined; if (value != null) { const picked = value; .. }`.

Held by the corpus cases `catch_unwind_guard_returned` (its `gesture`
named by the binding, its `finished`, whose closure reads `finish`, by
the result) and `catch_unwind_nested_ok`, against native Rust; and the
mutations `ok-binding-alias-kept` and `ok-binding-clash-unchecked`
(`prepare/catches.ts`).

## Amended (2026-10-25): a `return` in the closure of a `try` that ends the function

A `return` in `catch_unwind`'s closure leaves the closure, so its body
was a closure called in the `try`, `try { (() => { .. return; .. })(); }`,
where it returned anywhere but at its end. **Where the `try` ends the
function, its last statement or the last of an `if`'s branch, a `try`'s
block or a `catch` that does, and each `return` of the closure gives no
value, its body is the `try`'s, each `return;` the function's**
(`prepare::catches::tail_tries`): after the closure returns, nothing
runs but the `try`'s `finally`, a guard's drop, as after the function's
`return`. React's `flushActQueue` writes it so, `try { for (..) { ..
return; } queue.length = 0; } catch (error) { .. } finally { isFlushing =
false; }`. Case A: the same runs, in the same order.

`docs/readability.txt`'s share of `return;` grows by that case's, 9.4
to 9.5 in 10,000 lines: a `return;` in a loop, the function's, as React
writes it, where it was a closure's.

Held by `test/corpus/catch_unwind_ok_returns.rs`'s `until_seven`;
mutations `tail-try-anywhere` and `tail-try-valued-return`
(`prepare/catches.ts`).

## Amendment: a flag out of the closure to return from the function is the `try` (2026-10-27)

React's dispatchSetStateInternal returns from inside its `try`:

```js
try {
  const eagerState = lastRenderedReducer(currentState, action);
  ..
  if (is(eagerState, currentState)) {
    enqueueConcurrentHookUpdateAndEagerlyBailout(fiber, queue, update);
    return false;
  }
} catch (error) {
  // Suppress the error. It will throw again in the render phase.
}
```

A closure can't return from its function, so its port gives a flag out
of `catch_unwind`, `if let Ok(true) = did_bail_out { return false; }`,
which printed `const didBailOut = $catchUnwind(() => { ..; return true;
.. return false; }); if (didBailOut.TAG === "Ok" && didBailOut._0) {
return false; }`.

**`const r = $catchUnwind(() => body); if (r.TAG === "Ok" && r._0) {
return v; }` is `try { body' } catch {}`** (`prepare/catches.rs`'s
`flagged_try`): each `return true` of the body the `if`'s `return v`,
its last, `return false`, gone, falling out of the `try` to what follows,
as the `Ok(false)` and the `Err` both went on to it; of `!r._0`, the other
way round. Only where the `if`'s `return` can't throw, which the `catch`
would catch where it ran outside the `Result`'s; where every other
`return` of the body gives the flag; and where nothing after reads `r`.
Case N: the same values returned, what was caught dropped as `$catchUnwind`'s
`Err` was.

Held by `test/corpus/catch_unwind_flag_return.rs`, against native Rust:
a flag, one the other way, one whose `return` may throw and one whose
`Result` is read after, kept; the guide's `dispatch`; mutations
`flagged-try-never`, `flagged-try-throwing-return`,
`flagged-try-negation-unseen` and `flagged-try-result-read-after`.


## Amendment: a loop's `try` of a closure that returns early (the user's decision, 2026-10-28, wl4)

React's renderRootSync and renderRootConcurrent write `outer: do { try {
switch (..) { case X: ..; break outer; } ..; break; } catch (thrownValue)
{ handleThrow(root, thrownValue); } } while (true);`. Rust can't `break` a
loop from inside the closure, so the port's `'outer: loop { match
catch_unwind(|| { .. return; .. }) { Ok(()) => break 'outer, Err(e) =>
handle(e) } }` printed `try { (() => { .. return; .. })(); break; }`, an
arrow called in place; and tidy dropped `'outer`, which no jump named
before preparation inlined anything.

- **A loop label the program wrote stays through tidy** (`unlabeled`
  drops only MIR's, `bbN`, and `written labels`, first of the `jumps`
  stage, names each written loop as the program did); **preparation drops
  each that nothing names then** (`prepare/labels.rs`'s `loops`), so a
  label stays only where a jump needs it, as before.
- **A loop's `try` whose body is a closure called in place, then the `Ok`
  arm's jump, is the closure's body, each `return;` of it that jump**
  (`tried_jumps`): `break` as it is where nothing between the `return`
  and the `try` would take it, a `switch` a `break` or a loop either;
  else the loop's label, `break outer`; and where the loop has none, the
  closure stays, as no label the program didn't write is made. A `try`
  that ends the loop's body, the `Ok` arm's `continue` gone, is the same
  of `continue`. Not of a closure of a value, nor one labeling what the
  loop's label names.

Case N ([0262](0262-when-rust-and-js-disagree.md)): each `return` went on
to that jump, which runs where it did; what the closure declares is the
`try` block's, as the jump after it reads none of it.

Held by `test/corpus/tried_loop_jumps.rs`, against native Rust; mutations
`tried-jumps-off`, `tried-jump-taken-by-its-switch`,
`tried-try-ending-the-body-kept`, `written-loop-label-unnamed-kept`
(`prepare/labels.ts`) and `written-loop-label-dropped-in-tidy`
(`tidy/jumps/labels.ts`).

`docs/readability.txt`'s share of `return;` grows, 9.6 to 9.7 in 10,000
lines, by `tried_loop_jumps.rs`'s `kept`: the `return;` its closure
writes, inside the closure that stays where the loop has no label. It's
the program's own `return`, as it writes it; the closure around it is
what the user's decision keeps, no label the program didn't write.

## Amended (2026-10-31): the comments around a `match` of it (rd156)

React DOM's commitMount writes `// For object src, .. // TODO: ..` above
`try { setSrcObject(domElement, type, src); return; } catch (x) { // If
URL.createObjectURL() errors, .. }`. Its port's `match catch_unwind(..) {
Ok(()) => return, Err(_) => { // If .. } }` printed the comments above
the `match` inside the `try`, before the call, and the `Err` arm's after
the `catch {}`. The `try` was where the `Result`'s temporary is, which MIR
gives its `Ok` arm's place, past the call; it's now where the call to
`catch_unwind` is, where the `match` is written, so what's above the
`match` is above the `try`, and a comment in the closure, before what it
calls, still in it (ADR 0444, amended). And an `Err` arm of only
comments, the comment-only block tidy puts after the `if` of an `Ok` arm
that leaves (ADR 0753), is the `catch`'s: `} catch { // If .. }`. Case
([0262](0262-when-rust-and-js-disagree.md)): N, comments' places. Held by
`test/lowering-unwind-arms.test.ts`' "comments around a match of
catch_unwind are where they're written", run in JS, and the mutations
`unwind-try-at-ok-arm` and `unwind-comment-only-err-after`.

## Amendment: a flag's `try` keeps its comments where they're written (rd634, 2026-11-02)

Case ([0262](0262-when-rust-and-js-disagree.md)): N, comments only.
React DOM's FizzInstructionSetShared writes `try { if (..) { ..; return;
} // Fall through to reveal. } catch (x) { // Fall through to reveal. }`.
Its port, `let started = catch_unwind(|| { if .. { ..; return true; } //
Fall through to reveal.\n false }); // Fall through to reveal.\n if let
Ok(true) = started { return; }`, printed both comments above the `try`:
the flag's `try` was placed where the `if` it folds is, after its body,
so each comment before that `if` was before the `try` as printed, the
one before the closure's `false` too, though the `return` it was, gone,
left where its comments end at the body's end (`arms_where_returned`).

**The flag's `try` is where the closure is made** (`flagged_try`, the
`let` of `catch_unwind`'s), so what its body holds is inside it, the
`false`'s comment at its end; and **a comment right above the folded
`if let Ok(true)` is the `catch`'s**: that `if`'s only arm is the flag's
`return`, so what it says is of the other way, the `Ok(false)` that
falls out of the `try` and the `Err` the `catch` is, upstream's place
for it. Only there: a comment above an `if let Ok(..)` whose arm does
something of its own is about that arm as much, and stays where it is
(`try_catch`'s forms are as before). The `catch` is a block of only that
comment, empty without one (ADR 0753); a flag's `catch` has no
statements, as its `Err` does nothing.

Held by `test/lowering-comments-inside.test.ts`'s "a comment before a
flagged catch_unwind's false is at its try's end, one above its test in
the catch", run, and the mutations `flagged-try-catch-uncommented` and
`flagged-try-at-its-test` (`prepare/catches.ts`).

## Amendment: a drop that panics while a panic unwinds aborts (#847, the user's decision, 2026-11-06)

Case ([0262](0262-when-rust-and-js-disagree.md)): R. Results change:
native Rust ends the process, where the JS went on with the drop's panic
in place of the first, which `catch_unwind` caught, so what ran after,
and what it printed, differed.

```rust
let r = panic::catch_unwind(|| {
    let _a = Bomb("a");
    let _b = Bomb("bomb"); // its drop panics
    panic!("second");
});
println!("never {}", r.is_err());
```

Rust drops `_b` as `second` unwinds; its panic can't unwind through the
cleanup, so std writes `thread caused non-unwinding panic. aborting.` and
aborts: `_a` is never dropped and `never` never printed.

- **What runs as a panic unwinds, a cleanup, is `$cleanup(() => { .. })`
  where it may throw**: a throw inside it is `$abort(second)`, which writes
  std's line to stderr and ends the process with 134, the code a shell
  gives SIGABRT, where the host has a process (`process.exit`). The
  line is std's own, which it writes whatever the hook; the hook's,
  `panic in a destructor during cleanup`, is the `$Abort`'s message, as
  rust-js has no hook (`set_hook` is refused, above).

  ```js
  try {
    throw new Error("second");
  } catch (error) {
    $cleanup(() => {
      console.log(`drop ${c}`);
      ..
    });
    throw error;
  }
  ```

- **A `finally` holds only what can't throw**, as it runs on either path
  and can't tell a drop on the way on, whose panic goes on, from one as a
  panic unwinds, which aborts. A drop that may throw after a body is
  `try { body } catch (error) { $cleanup(() => { drop }); throw error; }
  drop`: tidy's scope merges (`finally_holds`) and every `finally` a drop
  is emitted in, drop glue's parts, `map_or`'s fallback, `xor`'s pair
  (`drops::dropped_after`), and tidy's last pass, "guarded cleanups", puts
  each rethrowing `catch`'s drops in `$cleanup`. A drop that can't throw
  keeps its `finally`, as before. Inside one drop, its parts still drop in
  turn as one of them panics, as Rust's drop glue does, and the abort comes
  where that panic leaves the drop: `drop first, bomb 2, drop last`, then
  the abort.
- **An early return drops where it leaves**: a scope no `finally` holds
  is MIR's block of it, `bb19: { .. break bb19; .. } drop; return;`, a
  label the program never wrote. Tidy's "dropped at breaks" pass, after
  "scoped loops" (`unwinds::dropped_at_breaks`), writes each `break` of the
  block as what follows it: the drops each `catch` in the block runs as a
  panic unwinds (`dropped_last`, so none of the program's own code is
  copied), each one no `finally` holds, then the `return`, or a throw the
  drops' `catch` guards, `try { throw x; } catch (error) { $cleanup(..);
  throw error; }`, React's ReactChildFiber's. A `break` inside a `try`,
  whose `catch` or `finally` would run again as it drops, stays, and so
  does the block. So `if (..) { const result = ..; drop; return result; }`
  where Rust returns; React's port has no compiler label left (#861).
- **The runtime's own cleanup is the same `$cleanup`**: `$dropsUnwinding`,
  which `$unwinds` and `$inTurnOf` run (ADR 1092), where #817's
  `$unwindingThrew` kept the first error.
- **No `catch_unwind` catches an abort**: `$catchUnwind` throws a `$Abort`
  on, and once aborted, `$cleanup` runs nothing more.
- **The browser's limit**: it has no process to end, so `$abort` throws a
  `$Abort` past `$catchUnwind` and the runtime's catches; a `catch_unwind`
  lowered to JS's own `try { .. } catch { .. }` (ADR 0637) catches it, as
  any JS `catch` would. Rust's own abort there is wasm's `unreachable`, a
  `RuntimeError` a JS `catch` catches too.
- **What it costs**: where a drop may panic, a dyn's or a generic's, an
  iterator's close, a `$dropItems` of items whose drop may, its `finally`
  becomes the `catch` and the drop after, and the cleanup `$cleanup`'s:
  311 of the corpus's 1,804 snapshots, 300 longer. What's known never to
  throw keeps the `finally`: a `RefCell` guard's `$unborrow`, now one of
  the runtime's helpers that never throw, and an `Rc`'s `$rcDrop` of a drop
  that can't, which drops 11 snapshots' `try`s besides. And ADR 0902's
  analysis knows more never unwinds: a `Cell`'s or a thread-local
  `Cell`'s `set` or `replace` of a value that drops nothing (a module's
  `let` written, as rust-js runs it), a closure written in a generic
  function (its drops of what's of a type parameter ways to unwind), and
  so the call of such a closure. What's left, a generic `Drop` impl judged
  for each use (the guide's `Defer`), a `Cell` of an `Option` of what drops
  nothing, a `Cell`'s `get`, a quiet `$dropItems` and an iterator's close
  of quiet items, is #861's. React's port pays 164 guards at its first
  measure.

The corpus judges an abort as it judges a panic: `//@ run-abort`, native
Rust's SIGABRT and the JS's exit 134 each `{ exit: 134 }`
(`test/programs.ts`' `execute`), what each printed compared byte for byte.

The paths a drop that can't throw still takes, one `finally` of its scope,
stay held by tests whose guards' drops can't throw, a print or a `Cell`'s
set of what the guard holds, beside those whose drops now may: the
lowering tests' `runAtDepth`, `quiet_frame`, `quiet_guarded`, `counted_root`
and a scope of a closure guard that can't throw, and the corpus cases'
`*_restored`, `until_seven_finished`, `kept_said`, `parse_at_depth`, `initialize_by_hand`,
`handle`, `twice_d`, `labelled_rsum`, `looped_gen`, `summed` and a loop of
`ReadsPlain`. Three paths no program reaches until #861 makes more drops
quiet keep their code but lose their mutations, which #861 brings back:
`run-reads-what-drop-reads` and `leading-assignment-taken`, of a loop's
drop of what it steps (a generic's or an iterator's close), and
`guard-copy-defined-lost`, of a guard taken apart whose drop `take()`s an
`Option` (rd523).

Held by the corpus cases `unwinding_drop_aborts_locals` (and a drop that
catches its own panic as it unwinds, which goes on),
`unwinding_drop_aborts_field` (no `catch_unwind`), `unwinding_drop_aborts_vec`
`unwinding_drop_aborts_iter` and `unwinding_drop_aborts_assert` (a bound's
panic, an `Rc`'s drop), against native Rust in both builds;
`test/lowering-unwinds.test.ts`, "a cleanup that throws aborts, past
catch_unwind", of a host whose `exit` doesn't end it;
tidy's contract "what unwinds is guarded", checked of every body in a debug
build; `test/architecture-iterators.test.ts`, "what the lowering runs as a
panic unwinds is guarded by drops/parts.rs alone"; and the mutations of
`abort-cleanup-throws-on`, `abort-exit-unasked`, `abort-line-unwritten`,
`abort-caught-by-catch-unwind`, `abort-cleanup-runs-after`
(`runtime/unwind.ts`), `cleanup-unguarded`, `dropped-after-finally-unguarded`,
`rc-drop-quiet-of-any` (`lower/drops/parts.ts`), `finally-holds-what-may-throw`,
`cleanups-left-unguarded`, `tested-unwrap-drops-unguarded`,
`dropped-at-breaks-off` (`lower/mir/tidy/unwinds.ts`) and `assert-drops-unguarded`
(`lower/mir/structure.ts`); tidy's contract is held by its unit test,
`what_unwinds_is_guarded`.

## Amendment: drop glue's later parts run flat as a part's panic unwinds (#865, 2026-11-07)

Case ([0262](0262-when-rust-and-js-disagree.md)): R. Results changed:
where a part of drop glue panicked and a later part panicked too, the JS
dropped the parts after that one before the abort, where Rust aborts at once.

#847's drop glue, `each_dropped`, wrote each part that may panic as
`dropped_after(part, rest)`, `try { part } catch (error) { rest; throw
error; } rest`, where `rest` was the later parts' own glue, each part in
its `try` again. So the `catch` of a part ran each later part guarded, and
one that panicked as the first unwound didn't stop the parts after it:
`drop 1`, part 2's panic, part 3's, then `drop 4` and the abort, where Rust's
cleanup of drop glue aborts at part 3's panic. And the copy of `rest` in
each `catch` doubled the JS with each part: num-traits' 20-tuple
`Bounded`'s `$drop` was 2^20 copies, `bounds.js` 240 MB, 172 s and 11.6 GB
to compile, where it took 9 s before #847.

**As a part's panic unwinds, the later parts run flat, each as it is**,
`try { a } catch (error) { b; c; throw error; } try { b } catch (error) {
c; throw error; } c` (`dropped_after_unwinding`), where tidy's "guarded
cleanups" makes the `catch`'s drops `$cleanup`'s, so a later part's panic
aborts there, as Rust's does. A drop of one or two parts reads as before;
of more, the JS grows as the square of its parts.

Held by `test/corpus/drop_glue_second_panic_aborts.rs` (`//@ run-abort`, no
`drop 4`) and `drop_glue_of_many_parts.rs`, a tuple's, a struct's and a
generic 20-tuple's drop where each part in turn panics, against native Rust
in both builds; `test/lowering-drop-guards.test.ts`' 12-tuple, whose JS
is bounded; and the mutation `drop-glue-rest-each-in-its-try`
(`lower/drops/parts.ts`). std's `iter.js` is made again: `scan`'s,
`flatten`'s and `flat_map`'s drops of three parts run their last two flat.
num-traits compiles in 9 s again (DEVELOPMENT.md's speed-ups).

## Amendment: a drop's cleanup is guarded where it's made (#938, 2026-11-08)

Case ([0262](0262-when-rust-and-js-disagree.md)): R. Results change:
native Rust aborts as a second drop panics while the first unwinds, where
the JS went on and `catch_unwind` caught it.

`dropped_after_unwinding`'s `catch` ran its drops as they were and left
their guard, `$cleanup`, to tidy's "guarded cleanups", its last pass. An
arrow made outside a function's body never passes through it: a closure's
drop of its captures, `$holding(.., () => { try { a } catch (error) { b;
throw error; } b })`, and each std struct's `close` in `std/iter.js` (20
of its `catch`es, 26 counting those made of other parts). So a `map(f)`
whose `f` held two values that each panic as they're dropped printed
`drop 81`, `drop 82`, `true` where Rust aborts after `drop 82`, in both
builds; with `--std-iterators`, so did a `filter(p).map(f)` collected,
whose `map`'s `close` drops `f` as `p`'s drop unwinds.

**Drop glue's `catch` is guarded where it's made**: `dropped_after_unwinding`,
of `each_dropped` and the glue lowered from rustc's shim, gives its drops
to `drops::cleanup`, the one guard, so a closure's drop of its captures
and a std struct's `close` abort where they're made. An `inspect(f)`'s
drop of its item as `f` panics, which wrote its own `catch`, is
`drops::dropped_unwinding`'s, which already guarded it. A function body's
`dropped_after`, a write back as a call unwinds say, and the MIR's
cleanups (`mir/drops.rs`) stay as they were: tidy's last pass guards them
by the same owner, and guarded earlier, tidy's naming passes read a
closure where the drops were and name what it captures apart
(`generic_mut_moved_out_other_ways.js`'s `s` became `s$1`, `s$2`).

`test/architecture-cleanups.test.ts` holds it: the lowering's files that
make a `catch` are a list with their reasons, drops/parts.rs's the only
ones outside a body's MIR that run a drop, and every `catch` in
`std/iter.js` that throws on what it caught runs only `$cleanup(..)` before
it. 19 corpus snapshots read `$cleanup(() => { .. })` in a `catch` of
drop glue where they read the drops as they were. Corpus cases
`closure_captures_dropped_after_a_sum_abort.rs` and, in iterator-forms'
few too, `collected_adapters_dropped_after_a_predicates_panic_abort.rs`.
Mutations: `drop-glue-cleanup-unguarded` (`lower/drops/parts.ts`),
`inspect-item-undropped-as-it-panics` (`lower/combinators/adapters.ts`, of `panic_part_way_drops.rs`);
`inspect-skipped` names its line as it now reads.

## Amendment: an owning iterator's count and a collect's cleanup abort where Rust's do (#862, 2026-11-08)

Case ([0262](0262-when-rust-and-js-disagree.md)): R. Results changed:
where a drop panicked and a second drop panicked as the first unwound, the
JS dropped more before the abort than Rust does. #847's iterator-matrix
rows, `drop+drop`, found two runtime paths:

- **A `Vec`'s, an array's or a `VecDeque`'s `IntoIter`'s `count` is std's,
  `len()`**, then the iterator dropped as one slice, front to back, where
  the JS folded it, dropping each item as it was counted. The drops are
  the same until one panics: Rust's slice glue then drops the rest in its
  cleanup, so the next to panic aborts there; the JS's fold closed what it
  had left as drop glue, which went on past that one. `vec![D(1), ..,
  D(5)].into_iter().count()` whose `D(3)` and then `D(4)` panic: Rust
  drops 1 to 4, the JS 1 to 5. `$closing`'s and `$slicing`'s `count` is
  now that (`count` of an `intoIter`, beside its `last`, #648), and the
  lowering asks it, `$count(it, drop)`, of a `Vec`'s `IntoIter` whose
  items have a drop (`drops_passed_as_slice`), where it was JS's `reduce`;
  generic code's `$count` finds it on what its caller gives.
- **What a collect a panic stopped holds drops as two frames' locals, not
  one value's parts**: what's left of its source, which std's fold or
  `extend` holds, and the `Vec` it collected, which `from_iter` holds, in
  that order (or the other, in place). `$collectUnwound` gave `$unwinds`
  one cleanup that ran them by `$inTurn`, drop glue's rule, so where the
  first's drop panicked the second was dropped before the abort:
  `rev().map(..).collect()` dropped the collected 105 and 104 after the
  source's panic, where Rust aborts. `$collectUnwinding(run, it, items,
  drop, restFirst)` now runs the collect's steps and gives them as two of
  `$unwinds`' drops, each `$dropsUnwinding`'s, so the first's panic aborts
  before the second. `$inTurn` stays drop glue's alone: a value's fields,
  a pair's, an adapter's parts.

One snapshot changes, `take_destructors.js`'s `count` of a `Vec`'s
`IntoIter`, the `reduce` now `$count(..)`.

Held by the corpus cases `vec_count_second_panic_aborts`,
`generic_count_second_panic_aborts` and `rev_map_collect_second_panic_aborts`
(`//@ run-abort`), against native Rust in both builds; the five
iterator-matrix rows taken off `test/iterator-matrix-known.txt`; and the
mutations `closing-slice-count-folds`, `slicing-count-folds`,
`slicing-count-leaves-the-rest` (`runtime/closing.ts`),
`collect-unwound-as-glue` (`runtime/fold_of.ts`) and `slice-count-folded`
(`lower/mir/iter.ts`).
