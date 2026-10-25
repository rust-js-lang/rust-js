# 0873. A function handed to JS carries no drop, and `catch_unwind` never unwinds

Status: Accepted. Amends [0521](0521-a-dyn-fn-carries-its-drop.md): a
closure made a `dyn Fn` only to hand it to a binding carries no drop; and
a closure that carries its drop, called by `FnOnce`, drops what it took in
its own body. Extends [0098](0098-destructors.md)'s "a value with a
destructor that JS holds is never dropped" to the functions a binding is
given, and [0660](0660-catch-unwind-catches-what-js-throws.md): a
`catch_unwind`'s call unwinds to nothing around it. Amended
(2026-10-25): an `unwrap()` whose panic drops what no scope's `finally`
does is its test, as an `assert!`'s is.

Case ([0262](0262-when-rust-and-js-disagree.md)): B, the guide's twelfth
difference, where JS gives a function it was handed back to Rust, which
drops it; A for the rest: what's dropped where Rust holds the function,
and a `catch_unwind` that never throws, is Rust's, the same either way.

## Context

React's `enqueueTask` takes a `task: () => void` and hands it to
`setImmediate`, or to a `MessageChannel`'s `onmessage`, never calling it
itself. Its port takes `task: Box<dyn Fn()>`, so `act` can pass a closure
that captures, and `shared` is a library whose callers' closures may hold
values with destructors (ADR 0521). The JS tracked drops it never ran:

```js
export default function enqueueTask(task) {
  let task$live = true;
  if (!enqueueTaskImpl) {
    let catchUnwind;
    try {
      catchUnwind = $catchUnwind(() => { /* .. */ });
    } catch (error) {
      $dispose(task);
      throw error;
    }
    if (catchUnwind.TAG === "Err") {
      const _err = catchUnwind._0;
      enqueueTaskImpl = (callback) => {
        // ..
        messagePort.onmessage = $dropping(
          () => { callback(); },
          () => { $dispose(callback); },
        );
        // ..
```

and `act`'s `queueSeveralMicrotasks` was `queueMicrotask($dropping(() => {
try { queueMicrotask(callback); } catch (error) { throw error; } }, () =>
{ $dispose(callback); }))`. Three things made it:

- **A closure made a `dyn Fn` carried its drop**, `$dropping(f, drop)`,
  wherever it went (ADR 0521). Given to `onmessage` or `queueMicrotask`,
  JS keeps it, and JS has no drop: nothing in Rust ever drops it again, so
  nothing runs what it carries.
- **`catch_unwind`'s call kept its unwind edge**, which rustc's MIR gives
  every call, so what's held around it was dropped in a `catch` of it,
  though it catches whatever unwinds through it, a panic or what JS threw
  (ADR 0660), and so never unwinds itself. Where a scope's `finally` took
  that `catch`, as most do, it read as one; where a drop flag kept the
  scope apart, `task$live`'s, it stayed, and kept the `if let Err(_err) =
  catch_unwind(..)` a `$catchUnwind` and its `TAG`, not a `try`.
- **A call whose unwinding drops nothing** was `try { .. } catch (error) {
  throw error; }`: the `FnOnce`'s own self, all given away to
  `queueMicrotask`.

Writing the corpus case found a wrong answer beside them: a closure that
carries its drop, made a `Box<dyn FnOnce()>` and called, dropped its own
self as its body ended by `$dispose(_1)`, a name its body hasn't: a
`ReferenceError` where Rust drops what it took.

## Decision

**A closure boxed where it's made, `Box::new(move || ..)`, and made a `dyn
Fn` straight into a binding's argument, or the `Some` of one, is JS's:
it carries no drop.** `ClosureFacts::handed_to_js`, a crate fact of the
coercions THIR makes, is the one answer; the coercion gives no
`$dropping`, and `Box::new` of it isn't refused as a function kept by what
it's given to. Only where Rust can't hold it again:

- **Nothing evaluated after it, before the call, may unwind**: a later
  argument a literal, a variable, a place of one or what's made of those.
  `run(Box::new(move || ..), checked(label))` panics in `checked` with the
  box made, which Rust drops as that unwinds: it carries its drop.
- **The binding gives back no function**: what it returns, or writes
  through a `&mut` it's given, holds no `dyn Fn`, closure, `fn`, `impl Fn`
  or type parameter. React's `widen`, a `this` binding, gives back the
  function it's given, Rust's again, which drops it.
- **A closure made a `dyn Fn` anywhere else too** carries its drop, as
  before; and a crate whose only such closures JS keeps has no `dyn Fn`
  that drops (`fn_dyns_drop`), so it disposes of none.

Where Rust still holds a function it will hand over, its drop runs as
Rust's does: a `Box<dyn Fn()>` parameter dropped as a panic unwinds before
the handoff, on a path that never hands it over (`if let Some(f) = ..
{ f(task) }`'s `None`), each a `$dispose(task)`.

**A `try` whose body can't throw, a `catch_unwind`'s, `try { result =
$catchUnwind(f); } catch (error) { drop(x); throw error; }`, that no
scope's `finally` took, is its body** (`unwinds::unthrown_catches`, after
the scopes are made): its `catch` never runs. `Stmt::cannot_throw` answers
which: what only makes values, and a `$catchUnwind(f)` of what can't
throw. The scopes are made first, so a `catch` of it a scope's `finally`
takes reads as one, as before. Preparation then writes the `match` as
`try` and `catch` (ADR 0637).

**A cleanup that drops nothing is no `try`** (`unwinding`): its `catch`
would only throw again.

**A closure that carries its drop drops what it took in its own body**,
an `FnOnce`'s, called, as one that carries none does: its captures, by
the names its maker gave them.

```js
export default function enqueueTask(task) {
  let task$live = true;
  if (!enqueueTaskImpl) {
    try {
      // ..
      enqueueTaskImpl = timers.setImmediate;
    } catch (_err) {
      // ..
      enqueueTaskImpl = (callback) => {
        // ..
        messagePort.onmessage = () => {
          callback();
        };
        channel.port2.postMessage(undefined);
      };
    }
  }
  if (enqueueTaskImpl) {
    task$live = false;
    enqueueTaskImpl(task);
  }
  if (task$live) {
    $dispose(task);
  }
}
```

`act`'s is upstream's, `(callback) => { queueMicrotask(() => {
queueMicrotask(callback); }); }`.

## Why

- **It's the same program**: a function JS keeps is never dropped by Rust
  again, so what it carries is never run; what Rust still holds drops
  where it did. `dyn_fn_handed_to_js` runs React's `enqueueTask` against
  native Rust, its task handed over by `setImmediate`'s stand-in and by a
  port's, a panic before the handoff, a path that never hands it over, a
  later argument that panics, `kept`'s and `put`'s functions given back,
  and `queueSeveralMicrotasks`.
- **`catch_unwind` never unwinds**: it catches what's thrown through it;
  in Rust a foreign exception aborts or is an `Err`, never unwinding to
  its caller.

## Alternatives

- **Removing `catch_unwind`'s unwind edge from the MIR**, as a binding
  that can't throw has none (ADR 0301): the same drops, but the `catch`
  a scope's `finally` started from was gone, so scopes began later or
  not at all: `catch_unwind_guard_returned`'s `try`, `catch` and `finally`
  became a `try` of each statement, and `catch_unwind_ok_returns`'s
  `return` in the closure an arrow called in place. Starting a scope at
  the call instead made one whose `finally` read what the call declares.
  Done after the scopes, as the JS's own `try`, what each took stays.
- **`$catchUnwind` among the helpers that never throw**: `may_throw` is
  read too as "runs none of the program's code" (`decisions.rs`'s value
  given before a `break`), which `catch_unwind` runs.
- **Every function given to a binding is JS's**: `widen` gives its back.
- **Keeping `$dropping` for a function JS keeps**: correct, but a drop
  nothing runs, which a person doesn't write.

## Consequences

- The readability baseline is blessed: the renamed share grew from 721 to
  721.2 in 10,000 lines because the no-op `catch (error) { throw error; }`
  blocks and the `try`s around a `catch_unwind` that can't unwind are gone,
  which shortens the corpus by more lines than it removes renames.

- The guide's difference twelve says it of functions too: a closure handed
  to a binding drops nothing of what it holds after, even where JS gives
  it back to Rust, through an export or another binding, which drops it.
- `closure_drops`' and `own_fold_through_adapters`' `catch (error) { throw
  error; }` are gone, and `adapter_drops`', `loop_drops`' and
  `map_into_iter_drops`' `try {} catch (error) { .. }`, whose empty body
  can't throw; no other corpus case changes.
- The guide's callbacks (`interop/bindings.md`) and `catch_unwind`
  (`interop/exceptions.md`) say it.
- Mutations in `scripts/mutations/lower/analysis/closures.ts`,
  `traits/dyns.ts`, `combinators/callbacks.ts`, `drops/parts.ts`,
  `mir/drops.ts`, `mir/tidy/unwinds.ts` and `js/stmt_walk.ts`.

## What holds it

- `test/corpus/dyn_fn_handed_to_js.rs`, `dyn_fn_handed_only.rs` and
  `dyn_fn_once_drops_itself.rs`, against native Rust.

## Amended (2026-10-25): an `unwrap()` that drops is its test

Case ([0262](0262-when-rust-and-js-disagree.md)): N. The same drops run,
then the same panic is thrown: a panic writes nothing as it's thrown
(ADR 0660), so what runs between its test and its throw, the drops, is
all anything sees, in the order Rust runs it.

React's `enqueueTask` calls `enqueueTaskImpl(task)`; its port,
`ENQUEUE_TASK_IMPL.get().unwrap()(task)`, whose `unwrap()` may panic
before the task is handed over, which Rust drops as it unwinds. The call
unwinding to that drop was its own `try`, as a call's is (`unwinding`,
`mir/drops.rs`), which no scope's `finally` took, the task moved by the
call after it; and the callee was held in its temporary, outside the
`try`, so that only the `unwrap()` is the drop's, not the call it hands
the task to:

```js
let task$live = true;
let tmp;
try {
  tmp = $unwrap(enqueueTaskImpl);
  task$live = false;
} catch (error) {
  if (task$live) {
    $dispose(task);
  }
  throw error;
}
tmp(task);
```

**Such a `try` of an `unwrap()` or an `expect("..")` of a `Some` that's
its value, read in place, is its test and panic, as an `assert!`'s are**
(ADR 0012): `if (option == null) { drops; throw new Error(message); }`,
then the option its value, read in place as tidy reads any temporary,
and the drop flags it leaves known settled again:

```js
if (enqueueTaskImpl == null) {
  $dispose(task);
  throw new Error("called `Option::unwrap()` on a `None` value");
}
enqueueTaskImpl(task);
```

`option::unwrap_tested`, of `$unwrap`'s owner, says what its call tests
and throws; `unwinds::tested_unwraps`, a pass of the `scopes` stage after
the scopes are made, writes it, so one a scope's `finally` drops around
stays `$unwrap(option)` in its `try`. It panics before the call, with
Rust's message, as ADR 0752 has `$unwrap(f)(x)` do. One of an option a
call makes, or of an `expect` whose message is read, a variable's, keeps
its `try`: read twice, its test and its value, the call would run twice, and the message
be read after the drops, which may change it. `rc_counted_uses`' two
`unwrap()`s of what a `Weak`'s `upgrade()` gives are tests too, and a flag
it never set live, `again$live`, is gone with its tests.

Held by `test/corpus/unwrap_dropping.rs`, against native Rust, and the
mutations `unwrap-tested-made` and `unwrap-tested-drops-lost`
(`mir/tidy/unwinds.ts`), `unwrap-tested-not` and
`unwrap-tested-flags-unsettled` (`mir/tidy/stages.ts`) and
`unwrap-tested-message-late` (`std_types/option.ts`).
