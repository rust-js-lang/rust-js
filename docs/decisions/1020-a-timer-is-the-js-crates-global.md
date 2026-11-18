# 1020. A timer is the js crate's global, which takes a function as it is

Status: Accepted (the user's decision, rd10; amended, rd10's follow-up: a closure the timer calls once drops what it took; amended, rd314, the user's decision: a window's `setTimeout` read as a value). Amends
[0102](0102-js-and-webapi.md) (timers are the builtins crate's): a
timeout's handler is TypeScript's `TimerHandler`, and a window's timers
are gone; [0229](0229-union-parameters.md) (a union parameter takes each
member as it is): a union's `Function` member may be any function of a
kind; and [0605](0605-an-arrow-that-passes-on-has-one-owner.md) (a closure
given to a once-callback that only passes its call on is the function):
a once-callback is what `recognition::function_kind` says is called by
`FnOnce`, a union's of one too.

Case ([0262](0262-when-rust-and-js-disagree.md)): none. What JS is given
is what the program gives, as it is; a value of `setTimeout` is the
global.

## Context

Two APIs set a timer. The js crate's `set_timeout(callback:
Box<dyn FnOnce()>, ms)` (ADR 0102) takes a closure; webapi's
`window.set_timeout(handler: &str, ..)`, generated from WebIDL, takes
text alone, as the generator drops a union parameter's `Function` member.
React DOM's ReactFiberConfigDOM installs the host's:

```js
export const scheduleTimeout = typeof setTimeout === 'function' ? setTimeout : undefined;
export const cancelTimeout = typeof clearTimeout === 'function' ? clearTimeout : undefined;
// ..
const blockingReady = Promise.race([.., new Promise(resolve => setTimeout(resolve, TIMEOUT))]);
```

and SchedulerPostTask reports an error with `setTimeout(() => { throw
error; })`. The js crate's held as a value already prints the first line.
What it takes was narrower than TypeScript's `setTimeout(handler:
TimerHandler, timeout?: number)`, `TimerHandler = string | Function`: a
`Box<dyn Fn()>` a function gives, `function::bind`'s, or a `&dyn Fn()`,
had to be boxed again, and text couldn't be given at all.

## Decision

**`js::set_timeout` is the one way a program sets a timeout, and its
handler is `TimerHandler`, each member as it is (the user's decision):**

```rust
pub fn set_timeout<H: IntoTimerHandler>(handler: H, ms: Option<f64>) -> &'static TimeoutId

#[cfg_attr(rust_js, rust_js::types = "string | Function")]
pub trait IntoTimerHandler: sealed::Sealed {}
impl<F: FnOnce() + 'static> IntoTimerHandler for F {}
impl IntoTimerHandler for &str {}
impl IntoTimerHandler for &String {}
```

- **A function is any function of no arguments, called once**: a closure,
  `js::set_timeout(move || resolve(()), Some(ms))`, a `fn`, a
  `Box<dyn FnOnce()>`, a `Box<dyn Fn()>` or a `&dyn Fn()`, each given as
  it is, `setTimeout(task)`. Text is code a browser runs as a script; Node
  and Bun throw a `TypeError` of it, as JS does there.
- **It's held as a value** by the instance its type says,
  `Some(js::set_timeout)` of an `Option<fn(Box<dyn FnOnce()>, Option<f64>)
  -> &'static TimeoutId>`, and asked of by any, as every generic binding
  is (ADR 0942): `js::is_function(js::set_timeout::<fn()>)`. React DOM's
  `scheduleTimeout` is `if js::is_function(js::set_timeout::<fn()>) {
  Some(js::set_timeout) } else { None }`, `typeof setTimeout ===
  "function" ? setTimeout : undefined`. `clear_timeout`, not generic, is
  held and asked of as before; so is `webapi::global::queue_microtask`,
  React DOM's `scheduleMicrotask`, `typeof queueMicrotask === "function" ?
  queueMicrotask : undefined`.
- **A union's `Function` member of a kind is called as that kind:**
  `recognition::function_kind` of a type parameter bound by a binding's
  union trait, one `#[rust_js::types]` types, whose blanket impl takes
  any function of a kind, is that kind (`member_function_kind`). So a
  closure given to `set_timeout` that only passes its call on is the
  function, `js::set_timeout(|| ping(), None)` is `setTimeout(ping)`
  (ADR 0605): `Taker::parameter` asks `function_kind` now, its own
  reading of a type parameter's bounds gone, one answer.
- **A window's timers are gone.** `window.set_timeout`, `clear_timeout`,
  `set_interval` and `clear_interval`, a second way, of text alone, are
  each a method of what nothing is, `G: TimerGone`, whose
  `on_unimplemented` says what to write: "a window's timers are gone: a
  timer is the js crate's global, `js::set_timeout(handler, ms)`, .."
  (`webapi/generate.ts`'s `TIMERS_GONE`), as `js::symbol::r#for` is gone
  (ADR 0634).
- **An interval is unchanged**, `set_interval(callback: Box<dyn FnMut()>,
  ms)`: a union of it would need a second trait for a function called
  again and again, and text, which nothing has asked of an interval.

## Costs and alternatives

- **The handler is generic, so `js::is_function(js::set_timeout)` names
  an instance:** `set_timeout::<fn()>`, any, as
  `js::is_function(js::object::prevent_extensions::<js::Unknown>)` does.
  The port's `js::is_defined(js::set_timeout)` becomes
  `js::is_defined(js::set_timeout::<fn()>)`.
- **A closure that holds a value with a destructor and only borrows it**,
  `move || { let m = &n; .. }`, isn't inferred `FnOnce` by rustc from a
  bound of a union, as it was from `Box<dyn FnOnce()>`. It was refused
  at first; it's called as Rust calls it now (amended below).
- **`Box<dyn IntoTimerHandler>`**, not generic, kept
  `js::is_function(js::set_timeout)`, but a `Box<dyn FnOnce()>` a program
  has would be boxed again, `Box::new(task)`, and a binding crate's union
  would have a second form beside 0229's `impl Into..`: rejected.
- **Dropping text**, `Box<dyn FnOnce()>` alone, kept everything as it was,
  but stays narrower than TypeScript's `TimerHandler`: rejected by the
  user's decision.

## Amended: a closure the timer calls once drops what it took (rd10)

The user's decision: keep the generic `set_timeout` and its turbofish,
and lift the refusal. Case (ADR 0262): results change, so Rust's
behavior is kept. Natively the handler is called by `FnOnce`: a closure
rustc infers `Fn` or `FnMut` is called through rustc's
`ClosureOnceShim`, which calls it by reference and then drops it, so
what it took is dropped as the call ends, after what its own body drops,
in the order it took them, and as it unwinds.

- **The fact:** `ClosureFacts::called_once`, the closures rustc infers
  `Fn` or `FnMut` made where they're given to a binding's parameter whose
  callback JS calls once (`Taker::parameter`'s `OnceCallback`, a type
  parameter of a union whose `Function` member is called by `FnOnce`).
  JS keeps it and calls it once, or never, as Rust's `call_once` takes it.
  Both refusals of a borrowing closure given away (`drops/facts.rs`, and
  `check_called_once` where its MIR call is lowered) ask it.
- **The body:** `Mir::called_once`, the shim inlined into the closure's
  own MIR: `_1` taken by value (`(*_1).f` read as `_1.f`), a `drop(_1)`
  before its return, and its cleanup's resume going on to drop it. The
  drop elaboration and structuring are the body's as for any closure
  rustc infers `FnOnce`, so the drops print where its body ends, in a
  `finally` where it can throw:

  ```js
  setTimeout(() => {
    const held = lease;
    console.log(`holding ${held[0]}`);
    console.log(`released ${lease[0]}`);
  }, 100);
  ```

- **Not lifted:** a closure given from a variable, `let f = move || ..;
  js::set_timeout(f, ms)`, which may be called by reference before (still
  refused, the guide's limitations), and a type parameter no bound says
  is called once.
- **Alternatives:** a JS wrapper calling the arrow and then its drop
  (`() => { f(); drop }`) adds an arrow per timer and a second drop
  writer beside the MIR's; lowering rustc's shim as a body of its own
  calls the closure from another function. Both rejected for the inlined
  shim, which reads as an `FnOnce` closure does.
- **Seen with it (#409, a wrong answer, fixed by 781c6cbdc, ADR 0521):** a
  closure rustc infers `Fn`, held in a variable and made a `Box<dyn
  FnOnce()>`, then called, never dropped what it took; it's
  `$droppingOnce(f, drop)` now.

## What holds it

- `test/js-timers.test.ts`: "a timer's handler is a function as it is, or
  text of code" (run, and text given as it is), "a timer and
  queueMicrotask are held as values where the host has them", "webapi's
  window.set_timeout is gone, an error naming js::set_timeout".
- `test/corpus/called_once_closure_drops_after.rs` against native Rust
  (borrowed, a `return`, an `FnMut`, an `FnOnce`, unwinding with and
  without a cleanup of its own) and `test/js-timers.test.ts`'s "a timer's
  closure that only borrows what it owns drops it after its one call".
- `test/lowering-passed-on.test.ts`'s once-callbacks: `js::set_timeout(||
  ping(), None)` is `setTimeout(ping)`.
- `test/compiler.test.ts`'s timers, `test/builtins.rs`; the scheduler's
  port, `test/ports/scheduler`.
- Mutations in `scripts/mutations/lower/recognition/traits.ts`,
  `scripts/mutations/lower/passed_on.ts`, and the `called-once-*` ones in
  `analysis/closures.ts`, `mir.ts`, `drops/facts.ts` and
  `combinators/callbacks.ts`.

## Amended: a window's `setTimeout` read as a value (rd314, the user's decision)

React's SchedulerPostTask holds the window's timer at load, `const
setTimeout = window.setTimeout;`, and its test replaces `global.window`
with its own, whose `setTimeout` logs the error a task threw: the
global's isn't the window's there. Case
([0262](0262-when-rust-and-js-disagree.md)): results change, so the
window's is kept. **Webapi's `Window::set_timeout` is back as a method
read as a value alone, `js::method((window, Window::set_timeout))`,
`window.setTimeout`, a `&'static dyn Fn(Box<dyn FnOnce()>, Option<f64>)
-> &'static TimeoutId`.** Its handler is a `G: TimerGone<fn()>`, which
only `Box<dyn FnOnce()>` is, so a call of it as a method,
`window.set_timeout(|| .., ms)` or of text, is the error naming
`js::set_timeout` still, one way to set a timer; its value, held, is
called as any function value is. `clear_timeout` and the intervals stay
gone (nothing reads them). A call given a `Box<dyn FnOnce()>` that's
already one, which `TimerGone<fn()>` takes, is refused too, with the same
message: the binding is `#[rust_js::value_only = ".."]`, which refuses
each call of it, saying what its value says (`bindings::value_only`, the
user's decision: one way to set a timer). `js::method` reads it, as
naming it is no call. `TimerGone` takes what it's a handler of,
`TimerGone<Of = ()>`, so no other timer takes a box.

Held by `test/js-timers.test.ts`'s "a window's setTimeout is read as a
value, the window's own" (run: a `window` of the test's own given the
handler) and "webapi's window.set_timeout is gone, an error naming
js::set_timeout" (a closure's call, and a `Box<dyn FnOnce()>`'s), and
the mutation `value-only-called` (`bindings/value_only.ts`). The message
is one, `webapi/generate.ts`'s `TIMERS_GONE_MESSAGE`, which writes both
`TimerGone`'s and the attribute's.

## Amendment: an effect's cleanup is called once too (#1881, 2026-11-15)

A closure that holds a value with a destructor must be made where JS sees
what it took, for a `let` or a call, or be one a binding calls once, made
where it's given (above). An effect's cleanup is neither: the effect gives
it back, `move || drop(listening)` of an RAII guard, as the deep-chain
harness's primitives stops listening to the window, so it was refused,
"a closure that holds a value with a destructor, made here".

**A closure given back, as its value, by a closure a binding takes whose
result JS calls once is called once** (`ClosureFacts::returned_once`):
`use_effect`'s `impl Fn() -> C` of a `C: Cleanup`, whose `FnOnce` member
says React calls it once, as `#[rust_js::types = "void | (() => void)"]`
on `Cleanup` makes it a union of a function of that kind
(`recognition::member_function_kind`). What it took is dropped as that
call ends: of one rustc infers `FnOnce`, its body's, and of one it infers
`Fn` or `FnMut`, its `call_once`'s, as above. Only its body's last
expression, not a `return`'s, which stays refused; and not what a binding
that doesn't call its result keeps, `use_memo`'s, refused as before.

What a cleanup only reads a field of, `move || println!("{}", guard.0)`,
it takes alone, by RFC 2229's precise captures: the guard stays the
effect's, dropped as the effect returns, before React calls the cleanup,
as native Rust drops it. Case ([0262](0262-when-rust-and-js-disagree.md)):
none, each drop where Rust's is; React calling a cleanup once is its
contract, as a timer's.

Held by `test/effect-cleanup-drops.test.ts`, real React mounting and
unmounting three cleanups, each log the native run's of the same
closures; and the memo's refusal. Mutations returned-once-closure-refused
(`drops/facts.rs`), returned-fn-closure-not-call-once and
any-binding-result-called-once (`analysis/closures.rs`);
closure-made-anywhere and returned-closure-made-refused retargeted.

## Amendment: a closure moved once into an `F: FnOnce` is called once too (#995, 2026-11-17)

Case ([0262](0262-when-rust-and-js-disagree.md)): none, Rust's behavior
kept; a refusal lifted.

A closure rustc infers `Fn`, holding a value with a destructor, bound by
`let` and then moved into a generic `F: FnOnce`, `let f = move || ..;
call_once(f)`, was refused: "giving `call_once`, which calls it once, a
function called by `&` that holds a value with a destructor". The same
closure written in place compiled, since rustc infers that one `FnOnce`
from the bound.

**Its body is its `call_once`'s too** (`moved_once`,
`analysis/closures.rs`), as one JS calls once is. It qualifies where the
variable's one use, by its own id, so a shadow of its name is another,
is that move into a parameter the callee calls by `FnOnce` alone. That's
`calls_param_once`, the reading `check_called_once` asks, however many
bounds deep (#2031). Rust's `FnOnce` of it calls it by `&` and drops it,
as the call ends or unwinds. Where the callee never calls it, its drop is
the `dropF` it's given. It's a fact of its own, `ClosureFacts::moved_once`,
and its body takes that shape only where it drops something. One that
drops nothing keeps its own body and its early `return`s, as React's
`reject` in ReactFlightClient.js does: a first draft reshaped it into
nested `if`s. Called or borrowed besides, it stays refused:
carrying the drop at its body's end would drop what it took at its first
call.

Corpus case `fn_closure_moved_into_a_once_callee` covers called,
uncalled, forwarded through a second generic `FnOnce`, given as an
`I: Is<F>`, a shadow of its name, and a panic, in plain, ITERATOR_FORMS
and METAMORPHIC=all. Compile-fail case
`fn_closure_called_then_moved_into_a_once_callee_refused`. Mutations
`moved-once-closures-unread` and `moved-once-other-uses-ignored`
(`lower/analysis/closures.ts`).

## Amended (2026-11-18, #2031): a `let`'s closure moved once is the owner's

A closure rustc infers `Fn` or `FnMut` bound by `let` and moved once into
a call (#995) is its `call_once`'s, its body dropping what it took, where
what generic code calls by value calls it and nothing calls it by `&`
(ADR 0470, amended: `analysis/closures/called.rs`), not where the
callee's bound reads `FnOnce`. Through a blanket impl of the crate's own
trait, `impl<F: FnOnce() -> u32> Thunk for F`, it now runs and drops
(`fn_closure_moved_through_a_blanket_impl_drops`).

## Amendment: a moved closure's other uses are its calls by `&` (2026-11-18)

`moved_once` asked that the variable's one use be its move. Since #2031's
owner, it also asks that no call by `&` calls the closure (`called.by_ref`).
That covers every use that calls it: `f()`, `let by = &f; by()`, and a
clone's call, which is of the same closure type. A use that doesn't call
it doesn't change what its `call_once` drops. The count of uses excluded
nothing more, and its mutation, `moved-once-other-uses-ignored`, survived
its case on main. It was retired with `--retire`'s evidence (ec1af1311),
and the count is deleted.
