# 0605. Whether an arrow only passes on what it's given has one owner

Status: Accepted. Amends [0198](0198-optional-handlers.md) and
[0279](0279-callbacks-passed-on.md). Amended below: `passed_on.rs` owns
the question with each position's rule, and a binding's `FnOnce` callback
is one ([0262](0262-when-rust-and-js-disagree.md), amended); and a
library's function is passed on as the crate's is; and a closure that
captures nothing as a `fn`, or lent as a `&dyn Fn`, is the function of
the crate's it only adapts; and a closure that passes on what it took by
value, then its parameters, is the crate's function bound, `g.bind(null,
a)`; and a closure of rest arguments that passes them all on to a
function's rest is that function.

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
- **The closure has a parameter of its own**: `move || shown(&fiber)` is a
  thunk, which JS writes as an arrow.
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
