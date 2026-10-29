# 1020. A timer is the js crate's global, which takes a function as it is

Status: Accepted (the user's decision, rd10). Amends
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
  bound of a union, as it was from `Box<dyn FnOnce()>`, so it's refused,
  as for any binding's type parameter that isn't bound by `FnOnce`, where
  it compiled before and dropped what it took after its body (the guide's
  limitations). Lifting it is a closure called once by its callee lowered
  as one that drops what it took, a separate change.
- **`Box<dyn IntoTimerHandler>`**, not generic, kept
  `js::is_function(js::set_timeout)`, but a `Box<dyn FnOnce()>` a program
  has would be boxed again, `Box::new(task)`, and a binding crate's union
  would have a second form beside 0229's `impl Into..`: rejected.
- **Dropping text**, `Box<dyn FnOnce()>` alone, kept everything as it was,
  but stays narrower than TypeScript's `TimerHandler`: rejected by the
  user's decision.

## What holds it

- `test/js-timers.test.ts`: "a timer's handler is a function as it is, or
  text of code" (run, and text given as it is), "a timer and
  queueMicrotask are held as values where the host has them", "webapi's
  window.set_timeout is gone, an error naming js::set_timeout".
- `test/lowering-passed-on.test.ts`'s once-callbacks: `js::set_timeout(||
  ping(), None)` is `setTimeout(ping)`.
- `test/compiler.test.ts`'s timers, `test/builtins.rs`; the scheduler's
  port, `test/ports/scheduler`.
- Mutations in `scripts/mutations/lower/recognition/traits.ts` and
  `scripts/mutations/lower/passed_on.ts`.
