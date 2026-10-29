# 0972. A function that reads its `this` is `js::function::with_this`'s

Status: Accepted (the user's decision, 2026-10-27, th1). Amends
[0761](0761-a-rust-js-class-is-a-constructor-function-and-its-prototype.md)'s
`self` as `this`, which a function made in a method now reads through a
`const`. Amended: its last parameter of rest arguments as a value is JS's
rest parameter (below). Amended by
[1009](1009-a-constructor-is-held-as-a-typed-class.md): held by
`Class::of`, its function is a constructor closed over what it captures.

Case ([0262](0262-when-rust-and-js-disagree.md)): none for the function:
its `this` is a parameter Rust sees, so what it reads is what JS gives it.
None for its name: Rust gives a closure no name, so the name a function
stored as a property has is the representation's, as a `let`'s arrow
takes the `let`'s by JS's own inference.

## Context

React's `initializeClassErrorUpdate` gives an error boundary a callback
that captures what it needs and reads the instance it's called on:

```js
update.callback = function callback() {
  logCaughtError(root, fiber, errorInfo);
  markLegacyErrorBoundaryAsFailed(this);
  this.componentDidCatch(error, { componentStack: stack !== null ? stack : '' });
};
```

`commitCallbacks` calls it as `callCallback(callback, context)`,
`callback.call(context)`. rust-js had no Rust for it: a closure is an
arrow ([0882](0882-a-field-of-statements-is-a-method.md)),
which never reads `this`; an object's methods
([0943](0943-an-object-of-methods-reading-this.md)) read `this` but capture nothing.
The nearest port read `fiber.stateNode` when the error was thrown, a
different instance wherever `commitCallbacks`' context differs, so
ReactFiberThrow was held.

## Decision

**`js::function::with_this(move |this, (a, b)| { .. })` is `function (a,
b) { .. }`**, its `this` JS's `this`:

- It's a binding, `link_name = "function"`, of a closure `F: Fn(Option<&'static
  Unknown>, A) -> R`, `A` a tuple of up to 12, giving a `Box<dyn Fn(..A) ->
  R + 'l>` (`js::function::Parameters`), which `js::unknown_of`,
  `function::call` and `function::apply` take, and Rust calls as any
  function, its `this` then `None`. An ordinary closure: rustc checks its
  captures, moves and borrows.
- Lowering (`binding_calls.rs`, `function_with_this`) makes the closure's
  arrow, `(this$, [a, b]) => ..`, a function of the tuple's items, `function
  (a, b)`, one it leaves out a parameter of its own, `(_, b)`; a tuple it
  reads whole is the array of them, `const pair = [pair$1, pair$2];`. Each
  read of its `this` is JS's, `js::read_as_this`, through each arrow in it,
  which sees its function's `this`; a function nested in it, another
  `with_this`'s, has its own, so it reads the outer one through `const
  this$ = this;`. A `this` the closure sets, `mut this`, is a `let` of its
  own, `let this$ = this;`, as JS's `this` can't be assigned.
- The closure is written in the call: one a variable holds is refused,
  naming the form.
- A class's method's `self` (0761) is read the same way: a function made
  in it reads it through `const counter = this;`, as it would read a
  handle's place (#359). A constructor whose instance such a function
  reads is refused.
- Preparation names it: stored as a property, `update.callback =
  function callback() { .. }`, where the property is a name a function
  may have, no word JS keeps, and one its body reads nothing by, which
  its own name would then be; a `let` of it is declared, `function inner()
  { .. }`, as an arrow of statements is (0882). Its `this` stays its own
  either way.

So the port writes:

```rust
update.callback.set(Some(js::unknown_of(function::with_this(move |this, ()| {
    let this: &'static ClassInstance = unsafe { js::cast(this) };
    ..
}))));
```

## Alternatives

- **An attribute on a `let` of the closure, `#[rust_js::this]`**: a
  closure's parameter would mean what its type doesn't say, and the
  closure couldn't be written in its call, as an attribute on an
  expression needs `stmt_expr_attributes` (ADR 0110).
- **A struct of the captures with a `#[rust_js::function]` impl**: not the
  Rust anyone writes for a callback, and a second capture analysis.
- **A wrapper of a closure a variable holds, `function (...a) { return
  f(this, a); }`**: a second form of the same function; the closure is
  written in the call instead.
- **Named by nothing, `update.callback = function () { .. }`**: correct,
  but not the name upstream writes, which devtools and stacks show.

## Consequences

- ReactFiberThrow's `initializeClassErrorUpdate` is upstream's `function
  callback()`, its `this` the instance `commitCallbacks` calls it on.
- `js::read_as_this` is the one rule for where a variable is JS's `this`,
  which `with_this` and a class's method and constructor ask.
- Preparation's arrow-to-method and arrow-to-declaration rewrites ask
  `js::reads_this`, which a function made in an arrow doesn't count for.
- Tested in `test/js-functions.test.ts`: `call`, `apply`, a method's and a
  bare call's `this`, nested functions, a class's method, a `this` set and
  a tuple read whole.

## Amendment (2026-10-28): its last parameter of rest arguments is JS's rest parameter

React's Scheduler wraps a callback so it's called on the wrapper's `this`
with what the wrapper's given:

```js
export function unstable_wrapCallback(callback) {
  return function () {
    ..
    return callback.apply(this, arguments);
  };
}
```

Its port, `function::with_this(move |this, (args,): (&'static dyn
Rest<dyn IntoUnknown>,)| function::apply(callback, this, Some(&args)))`,
printed `function (args) { return callback.apply(this, args); }`: one
parameter, the first argument alone, which `apply` can't read as a list.
A wrong answer, and not a JS caller's alone: the function's type, `dyn
Fn(&dyn Rest<T>)`, spreads a Rust call's arguments (ADR 0851), so
`wrapped(&(1, 2))` was `wrapped(1, 2)`, `args` `1` (gap rd8).

**The tuple's last item of rest arguments as a value is JS's rest
parameter**, `function (...args)`, `function (first, ...rest)`, as a
closure's is (ADR 0851): what its type says every call gives spread.
`function_with_this` asks `bindings::is_rest_value` of the tuple's last
type. One before the last isn't JS's, and its function's type, `dyn
Fn(&dyn Rest<T>, f64)`, is refused already. Case: a wrong answer fixed,
Rust's behavior kept ([0262](0262-when-rust-and-js-disagree.md)).

Held by `test/js-functions.test.ts`'s "a function with_this makes takes
rest arguments as JS's rest parameter": wrapCallback called as an
object's method, `this` and both arguments given on; a first parameter
and the rest, from JS and from a Rust call spread; and the refusal of one
before the last. Mutation `with-this-rest-one-value` (`binding_calls.ts`).


## Amendment (2026-10-28): a constructor closed over what it captures

React's `createSyntheticEvent(Interface)` makes a constructor per
interface (gap rd64). [1009](1009-a-constructor-is-held-as-a-typed-class.md):
**`Class::of(function::with_this(move |this, (a, b)| { .. this }))` is the
function as a constructor**, typed by its tuple and what it gives,
`Class<fn(A, B) -> T>` of a closure giving `this` cast to a `&'static T`.
A `let` of it named by `#[rust_js::name]` is declared by that name, as JS
names a constructor, `function SyntheticBaseEvent(reactName, ..) { ..
return this; }`, and keeps it where it's moved into another `let`; its
`prototype()` is `SyntheticBaseEvent.prototype`, which a struct literal of
`with_this` fields is assigned to as upstream's object of `function ()`s.
Case: none, the same function, `this` and `new` as JS's.

## Amendment: a call through a field gives no `this` (#402)

Status: the user's decision (2026-10-29), "land now, narrow later": this
as it is, and a follow-up keeping `s.f()` where no function that reads its
`this` can be the field's value, of each type, as `FnPointers` asks of a
fn pointer's.

Rust's `(s.f)()` of a `with_this` function a field holds gives it `None`,
but its JS was `s.f()`, which gives it `s`: a wrong answer, `true` where
Rust's is `false`. So was any call of a `dyn Fn`, a generic `F` or a fn
pointer that may be JS's, read as a member, `s.f`, or an item, `fs[0]`,
which JS gives as its `this`. **Such a callee is held first, `const f =
s.f; f()`** (`calls::called_unbound`), what's made before it flushed so
it's read where Rust reads it, before its arguments, so it's called with
none. A closure of the crate's, and a fn pointer that may only be a Rust
function (`FnPointers::rust_only`, React's `dispatcher.useState()`),
which never read their `this`, are called as they are, `p[0]()`. So is
each call of a program whose code may hold no function that reads it
(`FnPointers::may_read_this`): one that calls no binding's or library's
function, whose JS or `with_this`'s it could be given, and none JS may
call with a function value, a `pub fn` taking a `dyn Fn`, an `F` or a fn
pointer. A program of Rust alone reads `thenable.then(first)` as before.
One that may hold one holds each, React's `const enqueueForceUpdate =
this.updater.enqueueForceUpdate; enqueueForceUpdate(this, ..)`, where
upstream writes `this.updater.enqueueForceUpdate(this, ..)`, a cost the
follow-up narrows.

Case R: JS's `this` would be what Rust's isn't. Held by
`test/js-functions.test.ts`, "a dyn Fn a field or an item holds is called
with no this", run in JS: a field's, an item's, a generic pair's and a JS
fn pointer's, each `false`, and a Rust fn pointer's and the crate's
closure's called as they are, and a program whose one such function is
`with_this`'s. Mutations `unbound-*`, `dyn-fn-call-bound`,
`crate-closure-call-unbound` and `this-read-anywhere` (`mir/calls.ts`),
`js-fn-pointer-call-bound` and `rust-fn-pointer-call-unbound`
(`mir/dispatch.ts`), and `binding-functions-read-no-this` and
`given-functions-read-no-this` (`analysis/fn_pointers.ts`).

## Amendment: only where a with_this function of its signature is made (#404)

The user's decision, "By with_this made": #402's holding was decided for
the whole crate, so a crate that called any binding held every call
through a field, React's `const enqueueForceUpdate =
this.updater.enqueueForceUpdate; enqueueForceUpdate(this, ..)`. **A call
through a field is held only where a function `js::function::with_this`
makes in the crate's code may be its callee, one of its signature**: what
it's called with and gives, `A` and `R` of `with_this::<F, A, R>`, their
lifetimes erased, the args tuple and the output of a `dyn Fn`'s, a generic
`F`'s or a fn pointer's call (`FnPointers::may_read_this`, which keeps the
with_this functions the crate makes by their signatures; one made in
generic code may be any). Only such a function can show Rust a `this`,
and Rust says `None`. **Another, a JS function, is called through its
object, `this.updater.enqueueForceUpdate(this, ..)`, `handle.play()`, as
JS calls it**: that object is its `this`, as upstream's JS gives it. This
replaces #402's crate-wide answer (a binding's call, a function value JS
gives).

Held by `test/js-functions.test.ts`, "a dyn Fn a field or an item holds is
called with no this": a field of another signature than the with_this
function's, `o.g(1)`, its JS function reading `o` as its `this`; one made
in generic code; and a crate's closure and a Rust fn pointer of the
with_this function's signature, called as they are. Mutations
`with-this-unrecorded`, `with-this-generic-unrecorded` and
`with-this-any-signature` (`analysis/fn_pointers.ts`).

**A fn pointer's call is never held** (amended again, #404): a with_this
function is a `Box<dyn Fn>`, which no fn pointer can be, so
`ReactStrictModeWarnings.discardPendingWarnings()` and
`dispatcher.useHostTransitionStatus()` are called through what holds
them, though the crate makes a with_this function of their signature.
`test/js-functions.test.ts`'s `t.g()` holds it.
