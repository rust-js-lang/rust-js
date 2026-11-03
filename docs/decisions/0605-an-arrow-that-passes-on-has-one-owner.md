# 0605. Whether an arrow only passes on what it's given has one owner

Status: Accepted; amended (2026-10-31, the user's eta rule, rd284;
2026-11-04, a prototype's method that only passes its parameters on,
rd715). Amends [0198](0198-optional-handlers.md) and
[0279](0279-callbacks-passed-on.md). Amended below: `passed_on.rs` owns
the question with each position's rule, and a binding's `FnOnce` callback
is one ([0262](0262-when-rust-and-js-disagree.md), amended); and a
library's function is passed on as the crate's is; and a closure that
captures nothing as a `fn`, or lent as a `&dyn Fn`, is the function of
the crate's it only adapts; and a closure that passes on what it took by
value, then its parameters, is the crate's function bound, `g.bind(null,
a)`; and a closure of rest arguments that passes them all on to a
function's rest is that function; and a closure that ignores what it's
given, calling with nothing a function that reads no `this` and nothing
more, is that function wherever it's given (rd196, rd212, the user's
decision); and a binding that says it reads only its parameters is its
function as a value (rd648, the user's decision); and a closure bound so
needs no parameter of its own, and may spread its rest on (#502); and a
closure that only converts what it passes on to a function value it
captured is that function where it's made (decided on the user's behalf,
alien-signals run 10).

Case ([0262](0262-when-rust-and-js-disagree.md)): A: the output is
unchanged.

## Context

Two places asked one question, whether an arrow is just the function it
calls with its parameters:

- an array method's callback, `xs.map((x) => f(x))` as `xs.map(f)`
  (ADR 0279), `iterators::passed_on`, of an arrow that returns the call;
- an event's handler, `onClick={() => onClear()}` as `onClick={onClear}`
  (ADR 0198), `jsx.rs`'s own check, of a call of nothing, given back or
  not.

Each was right where it was, and they'd drift: found porting React's
scheduler, whose `onmessage = performWorkUntilDeadline` neither covers.

## Decision

**`iterators::passed_on` answers it, for both**: an arrow that passes on
what it's given, in order, to a function it doesn't name a parameter;
and, where nothing reads what the arrow gives, a handler's, one whose
call is a statement, `() => { f(); }`. Each position still says whether
it may: an array method's, where the arrow calls the crate's own (ADR
0279); an event's, where React ignores what it gives (ADR 0198).

Where else a closure may be its function, `onmessage = ..`, isn't decided
here: React's rules forbid some, `useEffect(onLog)` of a `useEffectEvent`.

## What holds it

- `test/architecture.test.ts`, "an arrow that passes on what it's given
  is passed_on's alone".
- ADRs 0198's and 0279's tests, unchanged; their mutations, moved to the
  owner.

## Amendment: the owner takes the position

`iterators::passed_on` answered the shape, but each position kept its own
rules beside it: jsx's handler had a second shape check, of an optional
handler, `(e) => { if (f != null) { f(e); } }`, and asked THIR whether
the call was Rust's (`calls_rust`), where an array method's read MIR's
spans, behind its own list of method names. Seven more functions checked
the shape of the arrows rust-js makes of a function as a value, each its
own way: a dictionary's `fmt`, a binding's `fn`, a trait method's and any
function's value, an impl's dictionary entry, a function's drops, and
`js::called_alone`; and preparation's forwarding function (ADR 0610).
Found deciding a binding's `FnOnce` callback, which would have been a
third position rule beside them, and a wrong answer they hid: `xs.map(|x|
many(&[x]))` of a function of rest arguments was `xs.map(many)`, which
counts the index and the array.

**`src/lower/passed_on.rs` owns it: `passed_on(f, taker, forwarding)`**,
each position a `Taker`, its rule there:

| Taker | Where | It may be the function of |
|---|---|---|
| `ArrayMethod` | `Taker::array_method(name)`, `map`, `filter` and the rest (ADR 0279) | the crate's function or a closure, or a generic `impl Fn` (case B) |
| `Handler` | `Taker::prop(name)`, `on` and a capital (ADR 0198) | any of the closure's, a `dyn Fn` prop's too; a statement of the call; an optional handler passed on |
| `OnceCallback` | `Taker::parameter(tcx, def_id, i)`, a binding's `FnOnce` parameter (ADR 0262, amended) | the crate's function or a closure; a statement of a call of `()` |
| `Made` | an arrow rust-js makes, which what's given it calls with its parameters alone | a function, a namespace's member too |
| `FnValue` | a function as a value, `.map(str::len)` (ADR 0538) | a function not a runtime helper, or an arrow of as many parameters |

- **What a closure passes its call on to is one fact, MIR's**: where MIR
  makes a closure of one call, `forwarded_to` says whether it calls the
  crate's function or a closure (`Own`), a generic `impl Fn` (`Generic`)
  or a `dyn Fn` (`Dyn`), and `mir.rs`'s `forwarding` keeps it by where
  it's written; every taker reads it. A binding's call has none, nor a
  function of rest arguments, which reads all it's given: the arity check
  the user asked for, where JS gives the function more than the closure
  did.
- **The shape is `js::passes_on`'s**, of a function's parameters and its
  body, rest arguments spread on too, which preparation's forwarding
  function asks as well: owned output can't ask lowering.
- **Each caller is a thin wrapper**: it says where the arrow is given and
  takes what `passed_on` gives.

### What holds it

- `test/architecture.test.ts`, "an arrow that passes on what it's given
  is passed_on's alone": the shape is matched in `js/stmt_walk.rs` alone,
  an optional handler's in `passed_on.rs` alone, the removed functions
  stay gone, the callers are the ones listed, and `forwarded_to` is asked
  where MIR makes a closure alone.
- `test/lowering-passed-on.test.ts`, and the corpus's
  `once_callback_passed_on.rs`, against native Rust: a once-callback of a
  function of the crate's is it, and a listener's, a JS function's, a
  rest function's and an array method's of a `dyn Fn` keep their arrows.
- `scripts/mutations/lower/passed_on.ts`, with the mutations moved there
  from `jsx.rs`, `iterators/eager.rs`, `iterators/ends.rs` and
  `mir/tidy/values.rs`.

## Amendment: a library's function is Rust too

`forwarded_to` said `Own` of the crate's functions alone, so a closure
that passes its call on to a library's function kept its arrow: React's
ReactStartTransition, `return_value.then(|_| noop(), report_global_error)`
of shared's `pub fn noop()`, was `then(() => { noop(); },
reportGlobalError)`, where upstream writes `then(noop,
reportGlobalError)`. Found porting React.

**A library rust-js compiled is the crate's kind of function** (`Own`):
its JS is compiled from its Rust as the crate's is, and takes the
arguments Rust gives it alone. `Foreign::is_library` says which crates
are: one rust-js built for this crate to use, not `@rust-js/std`, whose
hand-modeled calls may be a runtime helper's. The exclusions stay: a
binding's JS function, and a function of rest arguments, a library's
`#[rust_js::rest]` as its manifest says (`Foreign::takes_rest`).

Case ([0262](0262-when-rust-and-js-disagree.md)): the second, the
exception the user decided, widened only in what it may pass on to; it
reads no more of what JS gives than the crate's own function does.

Held by `test/cross-crate-items.test.ts`, "a once-callback that passes
its call on to a library's function is the function", which runs what it
wrote, and `scripts/mutations/lower/passed_on.ts`'s
`library-callee-not-rust` and `library-rest-callee-passed-on`.

## Amendment: the crate's own function's `FnOnce` callback

React passes a function to runWithFiberInDEV, which calls it with the
arguments after it: `runWithFiberInDEV(finishedWork, commitCallbacks,
updateQueue, instance)`. Its port's `run_with_fiber_in_dev`, the crate's
own function of an `impl FnOnce(A0, A1, A2, A3, A4) -> T`, is given a
closure, `|queue, instance, _: (), _: (), _: ()| commit_callbacks(queue,
instance)`, which printed `(queue, instance) => commitCallbacks(queue,
instance)`: `Taker::parameter` was asked only of a binding's arguments.

**A closure given to the crate's own function's parameter whose only
`Fn` bound is `FnOnce`, that only passes its call on to a function of the
crate's or a closure, is that function** (`mir_crate_call`), as a
binding's is (ADR 0262's exception): the function calls it at most once,
can't compare it, and gives it what its type says, of which the function
passed on reads its own parameters alone; the `()`s the closure leaves
out are `undefined` either way. An `impl Fn`'s, which the function may
call again or keep, keeps its arrow. Case N
([0262](0262-when-rust-and-js-disagree.md)).

Held by `test/lowering-passed-on.test.ts`'s "a closure given to the
crate's FnOnce callback that only passes its call on is the function",
run in JS, and `closures_of_their_value.rs` against native Rust; and the
mutation `crate-once-callback-kept` (`mir/calls.ts`).

## Amendment (2026-10-27): a closure that captures nothing as a `fn`

React's `ContextOnlyDispatcher` gives every hook one function, whatever
its arity:

```js
export const ContextOnlyDispatcher = {
  readContext,
  use,
  useCallback: throwInvalidHookError,
  useContext: throwInvalidHookError,
  ..
};
```

Rust can't make `throw_invalid_hook_error`, a `fn() -> !`, a `fn(A, B)
-> R`, so a port writes a closure for each field, `use_callback: |_, _|
throw_invalid_hook_error()`, and its JS was a method of each, `useCallback()
{ throwInvalidHookError(); }`.

**A closure that captures nothing, as a `fn` (ADR 0125), that only calls a
function of the crate's, or one rust-js compiled of a library's, with what
it's given, in order, or with none of it, where that function's signature
isn't the closure's, is that function**
(`Taker::FnPointer`, asked where `rvalues.rs` lowers the coercion): the
closure's MIR fact says its one call is the crate's (`Callee::Own`), whose
JS reads no more than it's given, so a caller's arguments it leaves out are
never read; and where the call's value is the closure's, or it never
returns, the function gives what the closure did. Rust compares no two
`fn`s reliably (a `fn`'s address isn't unique), so whether the field is
the same function as another, or as `throwInvalidHookError` itself, is
nothing a program can rely on. Case N
([0262](0262-when-rust-and-js-disagree.md)).

```js
export const CONTEXT_ONLY = {
  useCallback: throwInvalidHookError,
  useId: throwInvalidHookError,
  ..
};
```

**Only where the function's signature isn't the closure's**, other
parameters or another result (`Forwarding`'s `adapts`, which
`forwarded_to` reads off MIR's call): there Rust can't name the function
where the closure goes, and the closure only adapts it. Where Rust could,
`|context| read_context(context)`, the closure is a function the program
wrote to be its own, as upstream's DEV dispatchers wrap each of theirs,
`readContext(context) { return readContext(context); }`: it stays.

A closure whose call's value it doesn't give, `|a| { first(a); }`, a
binding's call, whose JS may read more than it's given, or a call with
anything else, stays the arrow too.

**So is one lent as a `&dyn Fn`**, React's `TransitionAwareHostComponent`
rendered as a component of two parameters, `render(&|_, _|
transition_aware_host_component())`, which is `render(transitionAwareHostComponent)`:
the same component each render, as upstream's, where an arrow made at each
call would be a new one. A reference to a closure of no size compares as
nothing reliable in Rust either. Not one an `Rc` or a `Box` owns as a
`dyn Fn`: an `Rc`'s allocation is its own, which `Rc::ptr_eq` compares,
so two of them are two functions as before. Held by "a closure that
captures nothing lent as a dyn Fn that only adapts the crate's function is
the function", run, the `Rc`s apart; mutations `owned-dyn-closure-passed-on`
and `lent-dyn-closure-kept-arrow` (`mir/rvalues.ts`).

Held by `test/lowering-passed-on.test.ts`, "a closure as a fn that only
calls the crate's function with what it's given is the function", run;
mutations `closure-fn-pointer-kept-arrow` (`mir/rvalues.ts`),
`fn-pointer-statement-kept`, `fn-pointer-wrapper-collapsed` and
`fn-pointer-always-adapts` (`passed_on.ts`).

## Amendment: a closure that passes on what it took is its function bound

React's mountTransition makes its `start` by partial application:
`const start = startTransition.bind(null, currentlyRenderingFiber, queue,
true, false);`, later called `start(callback, options)`. Its port can't
write `js::function::bind`: `Bound`'s rest is a `Box<dyn Fn(R..)>` of the
parameter types as inferred, each of one lifetime, where a stored start is
`dyn for<'a> Fn(&'a dyn Fn(), Option<&'a str>)`, and no generic impl makes
a higher-ranked `dyn` of type parameters ("implementation of `From` is not
general enough"). The port writes the closure, `move |callback, options|
start_transition(&fiber, &queue, true, false, callback, options)`, which
printed `(callback, options) => startTransition(fiber, queue, true, false,
callback, options)`.

The user decided (2026-10-27, h04c): **a closure that only passes what it
took by value, then each of its own parameters in order, to a function
item of the crate's (or of a library rust-js compiled) is that function
bound, `g.bind(null, a, ..)`**, under the conditions of the rule above:

- **The callee is a function item that reads no `this`** (`Forwarding`'s
  `item`, of `forwarded_to`'s item arm, not a closure's `Fn` call), named
  where it's called, `g(..)`, not a class's method, `x.m(..)`; and takes
  no rest arguments, so what JS gives a bound function more than the
  closure's parameters, `g` reads no more of than the arrow would.
- **Each value bound is one the closure took by value, or a constant**:
  `closures.rs` knows them where it makes the closure, `moved`, a capture
  neither borrowed (its place), a `&mut` (its variable) nor changed by the
  closure (a `let` of its own); one written again after it's taken is a
  copy already. `bind` reads each once, where it's made, which is what a
  closure that took it by value has, however the variable changes later.
  One at least is taken: a closure that captures nothing, `|x|
  noisy("take", x)`, stays the arrow a person writes. A value read when
  the closure's called, a module's variable, `PENDING.get()`, isn't one.
- ~~**The closure has a parameter of its own**: `move || shown(&fiber)` is a
  thunk, which JS writes as an arrow.~~ Superseded by the #502 amendment
  below: a thunk is bound too.
- Only `.name` (`"bound startTransition"`) and `.length` tell them apart,
  as of the function passed on itself above; each `bind` makes a function
  of its own, as each arrow is, so `Rc::ptr_eq` compares two as before.
  Case N ([0262](0262-when-rust-and-js-disagree.md)).

`passed_on.rs`'s `binds` decides it where the closure is made (the fact's
`binds`), and `bound` writes `g.bind(null, ..)` in tidy's last stage
(`finish/bound captures`), once nothing calls the arrow in place any
more: a closure an `Option`'s `map` calls where it's made is its body
there, `x != null ? noisy("find", x) : undefined`, not a bound function
called, `noisy.bind(null, "find")(x)`. `function::bind` stays the one way
to bind a `this`.

Held by `test/lowering-passed-on.test.ts`'s "a move closure that passes on
what it captured, then what it's given, is the function bound", run: a
borrow, a module's variable, a thunk, a closure called, a closure that
captures nothing each stay the arrow; and the guide's `dispatcher`.
Mutations `bound-any-variable`, `bound-thunk`, `bound-a-closure`,
`bound-of-nothing-captured`, `bound-never` (`passed_on.ts`).

## Amendment: a closure that captures nothing is made where it's given (2026-10-28)

Case ([0262](0262-when-rust-and-js-disagree.md)): N, a function that
reads nothing where it's made is the same function made anywhere.

React's createRootErrorUpdate gives `update.callback` a closure that calls
`run_with_fiber_in_dev(error_info.source.clone(), |root, error_info, _, _,
_| log_uncaught_error(root, error_info), &*root, &*error_info, (), (), ())`
(gap ce4 of the port). At the top of a function it was upstream's
`runWithFiberInDEV(.., logUncaughtError, root, errorInfo)`; in the
closure, it was `(root, errorInfo) => logUncaughtError(root, errorInfo)`:
reading what the closure captured, `&*root`, settled what was made before
it, the inner closure held in a `const` and read by its name, so the call
was given a variable, not the arrow its fact is of, and inlined after.

**A closure that captures nothing reads the same anywhere**
(`temporaries::reads_same_anywhere`, of a closure type of no upvars): it's
made where it's given, so its taker sees the arrow and its fact, and may
take the function it passes its call on to. Held by
`test/lowering-passed-on.test.ts`, "a closure given to the crate's FnOnce
callback that only passes its call on is the function", its `later`, run;
mutation `capture-free-closure-held` (`mir/temporaries.ts`).

## Amendment (2026-10-28): a closure of rest arguments that passes them all on to a rest

React's renderer configs without a feature, ReactFiberConfigWithNoPersistence
and the others, export one function as each the renderer lacks, which the
reconciler calls with that function's own arguments:

```js
function shim(...args: any): empty {
  throw new Error('The current renderer does not support persistence. ' + ..);
}
export const cloneInstance = shim;
```

A port's `pub const CLONE_INSTANCE: fn(&[Option<&Unknown>]) -> ! = shim;`
was `(x) => shim(...x)`, right for Rust, whose call through that type
gives one slice, and wrong for the reconciler's `cloneInstance(instance,
type, props)`, which spreads `instance`. A `fn` JS calls with its
arguments is one whose type says so, rest arguments as a value, `fn(&dyn
Rest<dyn IntoUnknown>) -> !` (ADR 0851), and its closure passes them on,
`|args| shim(args)`, `(...args) => { shim(...args); }`: right, and an
arrow where upstream has the function. `forwarded_to` gave no fact of a
call of a function of rest arguments, which reads all JS gives it.

**A closure whose last parameter is rest arguments as a value, which
passes them all on to a function's rest, is that function**, by the rule
above (`Taker::FnPointer`, an adapting closure that captures nothing):
`export const cloneInstance = shim;`. Its arrow gathers what JS gives,
however much, and gives all of it on spread, so the function is given
what the arrow would give it; `.name` and `.length` alone tell them
apart, as of every function passed on. Case N
([0262](0262-when-rust-and-js-disagree.md)).

- **The closure gathers them** (`forwarded_to` asks `gathers_rest`, the
  closure's last parameter `bindings::is_rest_value`): a closure of one
  array, `fn(&[T])`'s `(x) => shim(...x)`, takes one value, and a
  callback of its value, `xs.map(|&x| total(&[x]))`, `(x) => total(x)`,
  would be given `map`'s index too, which a rest reads. Each stays its
  arrow.
- **It gives them all on, in order**: `js::passes_on` matches the arrow's
  `...args` with the call's `...args`; one that gives less,
  `shim(&args[1..])`, `(...args) => shim(...$slice(args, 1))`, stays.

Held by `test/lowering-passed-on.test.ts`, "a closure as a fn of rest
arguments that passes them all on to a rest function is the function",
each run as a JS caller calls it, and the guide's `cloneInstance`
(interop/export-to-javascript.md); mutations
`rest-callee-never-passed-on`, `rest-callee-passed-on` and
`rest-gathered-of-any-last-parameter` (`passed_on.ts`).

## Amendment: a once-callback is what `function_kind` says is called once

[ADR 1020](1020-a-timer-is-the-js-crates-global.md): `Taker::parameter`
asks `recognition::function_kind` whether a binding's type parameter is
called by `FnOnce` alone, its own reading of the bounds gone; of a
union's whose `Function` member is called once, `js::set_timeout`'s,
too: `js::set_timeout(|| ping(), None)` is `setTimeout(ping)`.

## Amendment: a closure as a `fn` that only calls one host function is that function (rd158)

React DOM holds host functions themselves, `scheduleMicrotask =
queueMicrotask`, `localRequestAnimationFrame = typeof requestAnimationFrame
=== 'function' ? requestAnimationFrame : scheduleTimeout`. Rust holds them
as one `fn` type, so the port adapts the one whose signature differs,
`|callback| { js::set_timeout(callback, None); }`, which printed `(callback)
=> { setTimeout(callback); }`.

**A closure that captures nothing, made a `fn` or lent as a `&dyn Fn`,
that only calls one binding's global function, not a method, of no rest
arguments, with what it's given in order, its optional parameters `None`,
and gives what that call gives or nothing, is that function**, as one of
the crate's is (ADR 0605, amended): `setTimeout`. The user's decision
(2026-10-30), an exception of ADR 0262's second case: Rust's own calls of
the `fn` give it what the closure takes and read nothing it gives, so
nothing tells them apart; a JS caller of an exported value does, which
sees what the host function gives, setTimeout's id where the closure gave
`undefined`, and may give it more than the closure takes, setTimeout's
delay. A closure of the binding's own signature, `|cb| queue_microtask(cb)`,
is one the program wrote to be a function of its own, as written; the
binding itself, `webapi::queue_microtask`, is `queueMicrotask`. A
listener's, a handler's or a once-callback's taker keeps its rule.

`forwarded_to` gives a binding's call `Callee::Host`, which only
`Taker::FnPointer` takes; a statement alone, `{ g(a); }`, is passed on
only to a host function, whose value the closure discarded and which a
JS caller of an exported `fn` sees (one of the crate's function stays
its arrow, `|a| { first(a); }`). Held by `test/lowering-passed-on.test.ts`'s
"a closure as a fn that only calls one host function with what it's
given is that function" (run: `setTimeout` and `queueMicrotask` held as
one `fn` type), and mutations `host-callee-not-a-fn`,
`host-statement-unread`, `own-statement-passed-on`, `host-callee-a-handler`,
`binding-callee-passed-on` (`passed_on.ts`).

The guide's global-scope example, `queueTask`'s `typeof queueMicrotask !==
"undefined" ? queueMicrotask : runNow`, is upstream's now, where it was
`(callback) => queueMicrotask(callback)`.

## Amendment: a closure that ignores what it's given is the function it calls (rd196, rd212)

React DOM pings a commit's suspensey resources with one bound function,
`const ping = onUnsuspend.bind(state); loadingState.then(ping, ping);
preloadEl.addEventListener('load', ping);`, and its view transitions
pass on functions JS calls with what they ignore, `.then(layoutCallback,
layoutCallback)`. Rust's types need a closure of the callback's
parameters, `move |_| ping()`, which printed `() => { ping(); }`: a
listener's taker kept its arrow, and a `dyn Fn` callee was any JS
function's.

**A closure whose parameters are all unread, whose body only calls,
with nothing, a function that reads no `this` and none of what it's
given more, giving what that gives, is that function, wherever it's
given** (the user's decision, 2026-10-30, an exception of ADR 0262's
second case): `then(ping, ping)`, `addEventListener("load", ping)`. That
function, `Callee::Own`, is one of the crate's or of a library rust-js
compiled, of no rest arguments; a closure; a fn pointer only those fill
(`FnPointers::rust_only`, ADR 0330); or one of them bound,
`function::bind`'s, a `function::with_this` function's too, whose `this`
is the one bound, through `let`s that never change and a thread-local's
value, `ON_UNSUSPEND.with(|f| *f)` (`bound_own`, `own_target`).
`forwarded_to` says so of a closure, `Forwarding::ignores`, which each
taker takes, a binding's `Fn` or `FnMut` callback's too, now
`Taker::Callback`, which takes nothing else.

A JS caller may call that function with any `this` and arguments, as it
would the closure; the closure gives what it gives. Only its `.length`
and its identity tell them apart: a JS caller of an exported value sees
one function where Rust made a closure each time. A forwarding closure
is the function itself, so adding it twice to one target and event makes
one listener, where two closures were two, as `removeEventListener` given
it removes the one added (the identity a listener's taker was kept for,
which the user's rule takes). The user accepted this as part of ADR
0262's exception (2026-10-31). A binding's JS function bound, which may read what it's
given, keeps its arrow, and so does a `with_this` function unbound,
which reads the `this` it's called with.

Held by `test/lowering-passed-on.test.ts`'s "a closure that ignores
what it's given is the function it calls" (run: a bound `with_this`
function pinged by `then` and a listener) and its "a closure given to a
binding's FnOnce callback .." (`listenEvery(count)`, `keepAny(count)`);
mutations `ignoring-not-taken`,
`ignoring-item-given`, `bound-own-dyn`, `bound-host-own` and
`bound-with-this-not-own` (`passed_on.ts`).

## Amendment: where it's made, held by a `let`, and passing all on to a namespace's function (rd284, the user's eta decision)

The user's decision (2026-10-31, ADR 0262's exception, as rd196's):
**a closure that only passes all its arguments, in order, to a function
and gives back its result is that function.** Case
([0262](0262-when-rust-and-js-disagree.md)): rare and harmless, the
user's: only its `.length`, its identity and what more a JS caller gives
tell them apart.

- **Where it's made**, a closure that's the function it calls whoever
  takes it, one that ignores what it's given (rd196) or one of the eta
  rule's, is that function, `Taker::Closure` at the closure's aggregate
  (`mir/rvalues.rs`): a box's, a field's, a `let`'s value, not only a
  taker's. Not one an `Rc` or an `Arc` is made of, `Rc::new(|| one())`
  (`FnCx::counted_closure`): two such are two `Rc`s that `Rc::ptr_eq`
  tells apart, a result Rust's program reads (ADR 0959), which the
  exception doesn't take.
- **Held by a `let`**, it's that function where the variable is read
  (`held_functions`, a tidy pass after `aliases`): React DOM's default
  transition indicator's `let on_success = listener(|_: &Event|
  handle_navigate_complete());` is upstream's
  `navigation.addEventListener('navigatesuccess', handleNavigateComplete)`
  and its removal (rd284).
- **One that passes all it's given, in order, to a namespace's function
  through the value it's called on**, `move |f, a|
  noop_renderer.batched_updates(f, a)`, is that function, read where it's
  made, `batchedUpdates: NoopRenderer.batchedUpdates`, as createReactNoop
  writes it (`Forwarding::eta`): a namespace's function reads no `this`
  (ADR 1037), and reading the member once, where the closure's made, is
  Rust's: its `move` closure holds that value.

Two `Box<dyn Fn>`s of one forwarding closure that `ptr::eq` compares are
one function, as two listeners are: the identity the user's exception
takes (2026-10-31). An `Rc`'s or an `Arc`'s stays a function of its own,
as above, as `Rc::ptr_eq` reads its allocation.

The eta rule is narrower than the user's statement where an earlier
decision of this ADR holds: a closure of a function Rust could have
named there is one the program wrote to be a function of its own, React's
DEV dispatchers' `readContext(context) { return readContext(context); }`
(the amendment on `fn`s above), so a closure of the crate's own function
passing all on keeps its arrow outside a taker that takes it; and a
trait's method, a dictionary's, or a class's, which reads its `this`,
keeps it everywhere. The user confirmed the narrow rule (2026-10-31): a
wider one would lose the `this` and the extra arguments a dictionary's or
a class's method reads, a result that changes, and collapse React's own
explicit dispatcher wrappers, which upstream keeps.

Held by `test/namespaces.test.ts`'s "a closure that passes all it's
given to a namespace's function through a value is that function" (run,
a fresh instance's too), `test/webapi-events.test.ts` (both listeners
`handleNavigateComplete`) and the corpus's `rc_fn_ptr_eq`. Mutations
`eta-namespace-unseen`, `eta-member-unnamed` (`passed_on.ts`),
`closure-kept-where-made`, `counted-closure-made-function`
(`mir/rvalues.ts`) and `held-functions-kept` (`mir/tidy/stages.ts`).

## Amendment: a local function `with_this` makes that reads no `this` (rd284b)

React's ReactDOMDefaultTransitionIndicator's `handleNavigateComplete`
captures its state, `isCancelled` and `pendingResolve`, so the port makes
it a local function, `let handle_navigate_complete: &'static dyn Fn() =
function::with_this(move |_, ()| ..)`, a nested `function
handleNavigateComplete() {..}`. Its listeners, `listener(move |_: &Event|
handle_navigate_complete())`, were functions of their own, `function
onSuccess() { handleNavigateComplete(); }`, where upstream adds and
removes `handleNavigateComplete` itself. **A function `with_this` makes
of a closure whose `this` is `_` reads no `this` and nothing more than its
tuple, as a bound one, so a closure that ignores what it's given and
calls it with nothing, held by a variable that never changes, is that
function (`bound_own`, `ignores_this`), and a `let` of the closure reads
it in its place (`held_functions`, the fact `Forwarding::fixed`, where
it asked for an item before).** One whose closure reads its `this` keeps
its listener, which calls it of none, as Rust does. Case
([0262](0262-when-rust-and-js-disagree.md)): the user's exception, as
rd196's: only the listener's identity and `.length` tell them apart.

Held by `test/webapi-events.test.ts`'s "event methods and reusable
listeners emit direct JavaScript" (`localNavigationListeners`: added and
removed as `handleNavigateComplete`; `named`, which reads its `this`,
kept in its arrow). Mutations `fixed-local-function-unheld`,
`with-this-reading-this-held` and `with-this-never-own` (`passed_on.ts`).


## Amendment: a listener's taker is a closure's (2026-11-01)

A listener's, `Taker::Callback`, and a closure's where it's made,
`Taker::Closure`, took the same arrows, one that ignores what it's given
or passes it all on: two names for one rule, which the mutation
`ignoring-listener-kept` showed, surviving, as the closure's taker took
what the listener's no longer did. **A callback that isn't called once is
a `Closure`'s**, the one taker; the mutation is gone, the rule held by the
closure's (`ignoring-not-taken`).

## Amendment: a `dyn Fn` parameter each of the crate's calls fills with its own function (rd616 B)

React Flight's Node client reads a stream to its end with the function
each caller gives it, `startReadingFromStream(response, stream, onEnd)`,
whose `stream.on('end', onEnd)` hands the stream that function. Rust's
`stream.on(event::End, Box::new(move || on_end()))` of an `on_end:
Rc<dyn Fn()>` printed `stream.on('end', () => { onEnd(); })`: a `dyn Fn`
was any JS caller's function, `Callee::Dyn`, which may read its `this`
and what it's given. The client's callers fill it with a counter closure
or `close` bound to the response.

**A `dyn Fn` parameter is `Callee::Own` where each of the crate's calls
fills it with a function that reads no `this` and none of what it's given
more, and only those calls reach its function.** That's a function ADR
0577's `only_called` keeps: not exported, not a module's default, not a
trait's method, never named as a value. So what its calls give is all
it's ever given. The parameter is a binding that's never set again (not
`mut`), of a `dyn Fn` as it is, borrowed, boxed, or an `Rc`'s or an
`Arc`'s. `own_callbacks` (`passed_on.rs`) finds them once for the crate,
the first time one's asked, from each call `only_called` now keeps by its
expression, not by a count. What a call gives may be another such
parameter. All are taken to be one, and each that a call fills with
something else is put out, with what it fills, until none is: a
parameter's value always starts at some call that isn't a parameter's.
What fills one is `own_callback`: what `own_target` says by its type (a
function of the crate's, a closure, a fn pointer only those fill), or
what `bound_value` says, a function bound or one `with_this` makes of a
closure that ignores its `this`. Neither may read `js::arguments()`,
which counts what a JS caller gives it more: `own_target` now asks it of
a function (`reads_arguments`), and `bound_value` of a `with_this`
closure (`ReadsArguments::made`), as a bound function's call of it was
as wrong as a listener's. It sees through a block's value, a borrow, a
`Box::new`, an `Rc::new` or an `Arc::new`, an `Rc`'s clone, and `let`s
that never change. Not a `with_this` function that reads its `this`,
which a listener calls with its target as `this`, nor a thread-local's
value read as one. `bound_own` asks it of a `let`'s value too, so a
`let` of an `Rc::new` of a closure counts, as one of `function::bind`
did. A closure that calls a captured `Rc<dyn Fn()>` calls it through the
`Rc`'s `Deref` (`captured`, `is_rc_deref`), and a `Box<dyn Fn()>` through
the box's own `Fn` impl, so both are the variable the closure captured.

Case ([0262](0262-when-rust-and-js-disagree.md)): the user's exception,
as rd196's above. The closure ignores what it's given and calls a
function that reads no `this` and nothing more, so a JS caller sees what
the closure would give it. Only the listener's `.length` and identity
tell them apart.

Alternatives: asking it of every call where lowering sees it, as
`NulledParams` does (ADR 0861), lowers each function again once all its
calls are seen. Here the answer is in the HIR before anything is lowered.
A parameter-specific attribute would be a new source form for what the
calls already say.

Held by corpus `own_callback_params` (native run; `endedBy`'s `on(onEnd)`
of another such parameter, `later`'s boxed `on(task)`, `listenEnd`'s
borrowed one; kept: an exported function's parameter given on (`relay`),
a `mut` one set to it (`relayOr`), one of a function used as a value
(`listenValued`)) and `test/lowering-passed-on.test.ts`'s "a dyn Fn
parameter each call fills with the crate's function is that function"
(run: a listener's `startReading`; kept: `with_this` functions that read
their `this`, and a function and a `with_this` function that read
`js::arguments()`, run: each counts none). Mutations `own-callback-*`,
`own-target-reads-arguments` and `with-this-reading-arguments-own`
(`passed_on.ts`).

## Amendment: a statement of a function of `()` (rd660)

React's Flight host dispatcher lends each of its functions as a `&dyn
Fn` of the dispatcher's own signature, adapting one, `L: &|href, r#as,
options| { preload(Some(js::unknown(href)), Some(js::unknown_of(
r#as.to_owned())), options); }`, whose conversions print nothing. It
printed `L(href, as, options) { preload(href, as, options); }`, where
upstream writes `L: preload`: `forwarded_to` gave no fact of a statement
of the crate's function, as a closure that discards what its call gives,
`|a| { first(a); }`, isn't the function, whose value a JS caller would see.

**A statement of a function item's call that gives `()` or never
returns is a fact as the call given back is** (`gives_nothing`, of its
signature at what it's given): its JS gives `undefined` as the closure's
does, so each taker that takes a statement takes it, here the `&dyn
Fn`'s (`Taker::FnPointer`), under its rules, `adapts` among them. Case
([0262](0262-when-rust-and-js-disagree.md)): the user's exception this
ADR's `fn` amendment applies, no more: only `.length` and identity tell
them apart.

**A statement is never the function bound** (`Forwarding`'s `statement`,
which `binds` asks): `move |state| { on_action_success(&queue, &node,
state); }` stays upstream's arrow, `(nextState) => {
onActionSuccess(actionQueue, node, nextState); }`; bound, React's port
changed three such `.then` callbacks, ReactFiberHooks' and
ReactFlightReplyServer's two, from upstream's form.

**createBackingFormData's callback stays an arrow.** Its `formData.for_each(
|value, key, _| { let value = ..; append_backing_entry(&backing_store,
value, key); })` borrows the store, and upstream binds it,
`appendBackingEntry.bind(null, backingStore)`. A closure's `let`s that
print nothing, and a variable it borrows that JS assigns nowhere, could be
bound as soundly as what it took by value is; tried, React's port bound
some twenty other callbacks upstream writes otherwise: ReactChildFiber's
`existingChildren.forEach(child => deleteChild(returnFiber, child))`
twice, ReactFiberHooks' and ReactFlightReplyServer's `.then` arrows, and
Fizz's and ReactFiberConfigDOM's `forEach(flushResource, destination)`,
JS's `thisArg`, which a function of the crate's, reading no `this`, has no
form of. Upstream is no one way here, so which a borrowed capture is waits
on the user's decision.

Held by `test/lowering-passed-on.test.ts`'s "a closure lent as a dyn Fn
whose statement only passes on to the crate's function of () is the
function" (run: `preload`, `count`'s arrow of a function of a value, and
`onSuccess`'s arrow); mutations `own-unit-statement-kept`,
`own-statement-of-a-value`, `statement-bound` and `own-statement-passed-on`
(`passed_on.ts`).

## Amendment: a binding that reads only its parameters is its function (rd648, the user's decision, 2026-11-03)

React's ReactFiberCommitViewTransitions measures each clone with the host
config's function, `clones.map(measureClonedInstance)`. Its port binds the
function, `#[rust_js::link_name =
"./src/ReactFiberConfig.js#measureClonedInstance"] fn
measure_cloned_instance(instance: &'static Unknown) -> &'static Unknown`, and
writes `.map(measure_cloned_instance)`, which printed `clones.map((instance)
=> measureClonedInstance(instance))`. A binding as a value is the arrow of
its call (ADR 0039): an array method gives its callback the index and the
array too, and a JS function may read them, `["1", "2"].map(parseInt)`'s
radix. rust-js can't know a JS function's arity: this one is a host
config's, a different module for each renderer.

**`#[rust_js::reads_only_its_parameters]` on a binding says its function
reads only the arguments its parameters declare, and no `this`**
(`bindings::reads_only_its_parameters`): as a value it's the function
itself where its arrow only passes its parameters on as its call gives
them (`Taker::Made`), and an iterator adapter gives it its callback as it
is, as it does a function of the crate's (`is_called_as_written`):
`clones.map(measureClonedInstance)`, as upstream. A
binding that converts what it's given, a nullable parameter's `None`, keeps
its arrow. It's a promise made at the JS boundary, as `runs_nothing` is: the
program says what the JS function reads, which only its JS shows.

- **Refused where it can't be said** (`reject_unread_parameters`): on what
  isn't a binding, whose body is the program's and whose value is its
  function already; on a binding that isn't a function called as one, a
  method, whose `this` is what it's called on, a constructor or a
  property's read; and on one of rest arguments, which reads all JS gives
  it. Each error names the supported use.
- **Case** ([0262](0262-when-rust-and-js-disagree.md)): N. JS gives the
  function more than Rust does, which, as the attribute says, it never
  reads. Unsaid, the arrow stays, so `parseInt` keeps its.
- **Not the closure's form**: `.map(|i| measure_cloned_instance(i))` is
  clippy's `redundant_closure`, which idiomatic Rust doesn't write; its
  `forwarded_to` fact is a host function's still (rd158).

Held by `test/lowering-passed-on.test.ts`, "a binding that reads only its
parameters is its function as a value", run: `items.map(measure)` beside
an unmarked binding's arrow, whose JS function reads the index; and "a
binding's mark that it reads only its parameters is refused where it can't
be said", of a method, a function of the crate's and a variadic binding.
Mutations `parameters-read-unmarked-value` (`items.ts`),
`parameters-read-callback-wrapped` (`mir/iter/sources.ts`), and
`parameters-mark-of-a-body`, `parameters-mark-of-a-method` and
`parameters-mark-of-rest` (`bindings/effects.ts`).

## Amendment: a thunk, and a closure of rest arguments, bound too (#502, 2026-11-03)

alien-signals' `computed(getter)` returns `computedOper.bind(node)`, and
its `isComputed(fn)` is `fn.name === 'bound ' + computedOper.name`; so are
`signal`, `effect` and `effectScope`. Its port's `move ||
computed_oper(&this)` printed `() => computedOper(this$)`, whose `name` is
`""`, so `isComputed` of a computed was `false`: the rule above bound a
closure of a parameter of its own alone, and of named parameters alone,
so `move |value: &dyn Rest<dyn IntoUnknown>| signal_oper(&this, value)` of
a `#[rust_js::rest]` function stayed `(...value) => signalOper(this$,
...value)`.

**A `move` closure of no parameters of its own, or whose last is rest
arguments it spreads on to the function's rest, that passes on what it
took by value is that function bound**, by the rule above:
`computedOper.bind(null, this$)`, `signalOper.bind(null, this$)`. What
`bind` binds it reads once, where the closure's made, as the closure took
it; what JS gives the bound function more, the function, which takes no
rest unless the closure gathers it all (`gathers_rest`), reads no more of
than the arrow would, and a rest function given the rest the arrow
spreads is given the same. The shape is `js::passes_on`'s, which now
gives the values before what the arrow passes on (`passes_on_after`), so
the bound shape and the passed-on shape are one match. Case
([0262](0262-when-rust-and-js-disagree.md)): as the rule above, N: only
`.name` (`"bound computedOper"`) and `.length` tell them apart, which only
a JS reader, or `js::function::name`, sees; each `bind` makes a function
of its own, as each arrow is.

**Not a thunk handed on as a callback** (`FnCx::handed_on`, asked of a
thunk alone): a call's argument, boxed (`Box::new(move || ..)`, given or
returned), lent (`&move || ..`) or wrapped (`AssertUnwindSafe(move ||
..)`), a block's value too. JS writes each in place: Fizz's and
Flight's `scheduleWork(() => performWork(request))` and `scheduleWork(()
=> finishAbort(request, abortableTasks))`, their Node entries'
`createDrainHandler`, `return () => startFlowing(request, destination)`
of a returned `Box<dyn Fn()>`, createReactNoop's root's `getChildren() {
return getChildren(container); }` of a boxed field, and a hook's,
`useMemo(() => filterTodos(todos, tab), [todos, tab])`, which React's
lint asks be an inline function. One an `Rc` or an `Arc` is made of is a
function its holders share, bound: React's Flight clients'
`startReadingFromStream(response, stream, close.bind(null, response))`,
of their ports' `Rc::new(move || close(&closing))`; and so is one
returned or held as it is, alien-signals' `computed`, the scheduler's
`const nextTask = runTask.bind(null, ..)` of `let next_task = move ||
run_task(..)`. **Nor a closure whose parameters JS leaves out**, `move
|_| ping_task(&request, task)`, `() => pingTask(request, task)` as Fizz
writes it: it ignores what it's given, and stays the arrow it was. What a
closure of parameters of its own is stays as above.

The rd660 boundary stays, and is held where it leaked: a closure that
borrows what it passes is its place, never what it took by value, and a
statement of the call is never bound. A shared borrow of a local whose
callee writes through its `RefCell`, `|| entangle(&root, 9, 1)`, came to
`closures.rs` as the variable, not its place, and was bound with
parameters of its own already; what's bound is now what rustc says the
closure took by value (`analysis::taken_by_value`, `ByValue` or `ByUse`). Upstream writes thunks both ways, and no form of the Rust tells
`createDrainHandler`'s returned thunk from alien-signals' but the box: the
line is drawn where React's port and alien-signals agree with upstream
everywhere a thunk changes, and is a choice of output, not of meaning
(case N), which the user may move.

React's port, regenerated: its Flight clients' twenty-one `close(response)`
arrows are upstream's `close.bind(null, response)`, and SchedulerPostTask's
`nextTask` upstream's `runTask.bind(..)`; nothing upstream writes as an
arrow is bound.

Held by `test/lowering-passed-on.test.ts`, "a move closure of no
parameters, or of rest arguments, that passes on what it captured is the
function bound", run (`computed.name` is `"bound computedOper"`,
`isComputed` of it `true` through `js::function::name`; kept: one of rest
arguments given on as one array, a statement, one that captures nothing,
a borrow, of a local too, thunks given to a call, boxed, returned boxed,
lent or wrapped, and one of `|_|`; bound: one an `Rc` is made of given to a call), the earlier
test's `thunk`, now
`shown.bind(null, fiber)`, the corpus's `bound_thunks` against native
Rust, and the guide's `later`. Mutations `bound-of-parameters-alone`,
`thunk-handed-on-bound`, `thunk-made-an-rc-handed-on`,
`thunk-call-argument-bound`, `thunk-lent-bound`, `thunk-of-a-block-bound`,
`thunk-wrapped-bound` (`passed_on.ts`), `thunk-ignoring-bound`,
`bound-borrowed-capture` (`mir/closures.ts`) and
`rest-given-whole-passed-on` (`js/stmt_walk.ts`);
`bound-thunk` goes, the condition it held gone.

## Amendment: a prototype's method that only passes its parameters on (rd715)

React's ReactDOMRoot gives its hydration root's prototype a module
function itself:

```js
function scheduleHydration(target: Node) {
  if (target) {
    queueExplicitHydrationTarget(target);
  }
}
ReactDOMHydrationRoot.prototype.unstable_scheduleHydration = scheduleHydration;
```

Its port's method only calls it, `pub fn unstable_schedule_hydration(&self,
target: Option<&'static Node>) { schedule_hydration(target); }`, and
rust-js printed a function of its own, `ReactDOMHydrationRoot.prototype.
unstable_scheduleHydration = function (target) { scheduleHydration(target);
};`.

**A constructor function's prototype method ([0761](0761-a-rust-js-class-is-a-constructor-function-and-its-prototype.md))
that only passes its parameters on, past its `self`, in order, to a
function of its own module, of its parameters' and result's types, is
that function: `X.prototype.m = f;`** (`passed_on::forwarded_function`, the
namespaces' rule, [1037](1037-a-module-namespace-is-a-unit-struct-of-its-functions.md)).

- **Where its JS is that call alone** (`js::Class::forwarded`), of a
  function it names: any conversion the call needs, a box, a
  dictionary, is JS of its own, and it stays a function.
- **The function reads no `this`**, as a free function of the crate's
  never does, so the prototype's `this` changes nothing; nor
  `arguments` (`js::arguments()`), so what JS gives it past its
  parameters changes nothing either. Its result's type is the method's,
  so a statement, `f(x);`, gives what the method gives.
- **Of its own module's function, a declaration**: the prototype's
  property is read where the module loads, which a declaration's
  hoisting lets read anywhere. Another module's may not be loaded yet
  in a cycle, so its method stays a function of its own; and the
  function stays a declaration, not the variable it calls
  ([0610](0610-a-forwarding-function-is-its-variable.md)),
  nor a re-export in its place (rd575).
- **Not an ES class's**, whose methods are written in its body.

Case ([0262](0262-when-rust-and-js-disagree.md)): the forwarding
exception the user decided (rd196): only the function's `.name` and
`.length` tell it from the method's own, and the prototype's property
is one function either way.

Held by `test/method-forwarding.test.ts`'s "a prototype's method that only
passes its parameters on is the function it calls" (two such methods,
one of a result; one that does more and one of `()` whose function
gives a value, functions of their own; run) and "a prototype's method
passes its parameters on to a declaration its module's load can read".
Mutations `prototype-forwarding-unasked`, `forwarded-result-unasked`,
`prototype-forwarding-elsewhere` (lower/passed_on.ts),
`prototype-forwarding-unplaced` (lower/classes.ts),
`prototype-forwarding-printed-as-function` (to_oxc/statements.ts) and
`prototype-forwarding-load-unread` (prepare/forwarding.ts).

## Amendment: a closure that only converts what it passes on to a function value it captured (alien-signals run 10, decided on the user's behalf)

alien-signals' generic `computed<T>` (design B4) keeps its getter in an
erased node, `getter: Box<dyn Fn(Value) -> Value>`, of `Value = Option<&'static
Unknown>`, so it boxes a closure that converts the getter's types,
`Box::new(move |previous: Value| { let previous = previous.map(|previous|
Rc::clone(unsafe { js::cast::<&Rc<T>>(Some(previous)) }));
js::any_of(Rc::new(getter(previous))) })`. Each conversion is the value
itself in JS, so its arrow only passes on what it's given,
`getter: (previous) => getter(previous)`, where upstream, and the port's
erased version before B4, keep `getter`.

**A closure whose one call of a function value it captured, a generic
`impl Fn`'s, a `dyn Fn`'s or a closure's, isn't all it is, and whose
arrow, once lowered, only passes all it's given on to that function,
giving back what it gives (`js::passes_on`), is that function, wherever
it's made** (`FnCx::converting`, a `Forwarding` that's `eta`): its
conversions, `js::cast`, `js::any_of`, an `Rc` that isn't counted,
`Option::map` of one, are none in JS, which the arrow's shape says, as
any that is JS of its own, `n * 2.0`, `{ value, strong, weak }`, keeps
the arrow. Rust can name the function no other way there, as its types
aren't the function's; one of the function's own types, `move |x| f(x)`,
which Rust could have written `f`, is one the program wrote to be a
function of its own, and keeps its arrow (rd284's boundary, above).
Not where the crate reads a function's identity, `Rc::ptr_eq` of an
`Rc<dyn Fn>` it doesn't count or a raw pointer to one
(`Counted::points_at_functions`, ADR 1043's #500 rule), nor one an `Rc`
is made of where it compares them (`counted_closure`, ADR 0959); nor a
closure of `()` whose call gives a value it discards. Of a generic `impl
Fn`'s or a `dyn Fn`'s, which may be a JS function that reads more than
it's given, only where the closure is boxed into a struct's field where
it's made, `Node { getter: Box::new(..) }`, a `dyn Fn` the crate's code
calls with what the closure is given (`boxed_into_a_field`): one given
on, `xs.iter().map(|x| f(x))` of a `&dyn Fn(&u32)`, whose reborrow is a
conversion too, keeps its arrow, as the method gives the index and the
array (ADR 0279). Nor one whose call's
last argument its JS leaves out, `|n| tagged(n, None)`, whose function a
JS caller would fill it for, `[3, 4].map(tagged)`'s index (ADR 0443,
amended); nor a function that may read `js::arguments()` (rd509).

Case ([0262](0262-when-rust-and-js-disagree.md)): N for Rust, the user's
exception family for a JS caller. What tells them apart, none of which
Rust reads:

- **Its `this`**: the arrow calls the function with none, and the
  function, called as its holder's member, `this$.getter(this$.value)`,
  gets the holder, as upstream's `this.getter(..)` gives it. A JS
  function given as the getter may read it; Rust can't.
- **What more a JS caller gives**: the arrow passed on only its
  parameters. Rust calls the `dyn Fn` with those alone.
- **Its `.length` and its identity**: as a function passed on, and the
  identity only where nothing reads it, above.

Decided on the user's behalf (2026-11-04, alien-signals run 10, design
B4's option 1, the user asleep): the user's #500 rule, a closure that
passes on is its function where nothing reads its identity, extended to
the conversions a closure needs to pass on at all.

Held by `test/lowering-passed-on.test.ts`, "a closure that only converts
what it passes on to a function value it captured is that function"
(`getter,`, run; `(n) => scale(n * 2)` kept), and the corpus's
`closure_converting_to_a_function_value` (a generic `computed<T>`'s
`Rc::new(getter(previous.as_deref()))`, `next(n + 1)` and a statement of
`()` kept) and `closure_converting_identity_read` (two boxes compared by
`ptr::addr_eq`, `false`, each its arrow), against native Rust. Mutations
`converting-identity-unasked`, `converting-unit-discarding`,
`converting-left-out` (the corpus's `option_arguments`),
`converting-of-a-js-function-given-on` ("a once-callback is the function
only where .."'s `dyn_mapped`) and
`converting-unseen` (`passed_on.ts`).
