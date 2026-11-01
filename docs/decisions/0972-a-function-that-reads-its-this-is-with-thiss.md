# 0972. A function that reads its `this` is `js::function::with_this`'s

Status: Accepted (the user's decision, 2026-10-27, th1). Amends
[0761](0761-a-rust-js-class-is-a-constructor-function-and-its-prototype.md)'s
`self` as `this`, which a function made in a method now reads through a
`const`. Amended: its last parameter of rest arguments as a value is JS's
rest parameter (below); amended (#449): a place of its `this` is never read in a handle's accessors (below). Amended by
[1009](1009-a-constructor-is-held-as-a-typed-class.md): held by
`Class::of`, its function is a constructor closed over what it captures.
Amended: it takes as many parameters as a function, sixteen (rd400).

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
`test/js-functions.test.ts`'s `t.g()` holds it. What makes it so is the
types: `with_this` gives `<A as Parameters>::Function`, and **`Parameters`
is sealed, of a tuple alone** (`js::sealed::Tuple`), each a `Box<dyn
Fn(..)>`; an impl of another crate's, `type Function = fn() -> R`, is
rustc's error (`test/js-functions.test.ts`, "with_this's Parameters is of
a tuple alone"). A JS function given where a fn pointer goes is JS's own,
called through what holds it, as the user decided.

## Amendment: a field the crate fills with its own closures (#405)

Matched by signature alone, inputValueTracking's `tracker.stopTracking()`
was held, `const f = ..; f()`, as SyntheticEvent's with_this functions are
`() -> ()` too, though `stop_tracking: Box<dyn Fn()>` only ever holds the
crate's own `Box::new(move || ..)`. **A field of a `dyn Fn` whose every
write the crate makes is of a closure or a function of its own, or a box
or an `Rc` of one, and whose place is never lent mutably, holds no
with_this function** (`FnPointers::own_fields`, of each struct literal's
and assignment's write, and a `&mut` of the field taken): its call is
`tracker.stopTracking()`, as upstream writes it, whatever its signature.
The field is read of the call's own place, `(s.f)()`'s, or what a
temporary of it borrows (`calls::called_field`). A write of a variable,
of what a call gives or through a `&mut` may be any, and is held as
before; one another crate or JS makes of an exported struct is JS's,
called through its object, as the user decided. Held by
`test/js-functions.test.ts`, "a dyn Fn field the crate fills with its own
closures is called through its object", and the mutations `own-field-*`
(`analysis/fn_pointers.ts`).

## Amendment: it gives the JS function itself, `&'l dyn Fn(..A) -> R` (rd209)

React DOM's FragmentInstance keeps its `once` wrapper, a `function
(event) { .. listener.call(this, event) }`, as an
`EventListenerOrEventListenerObject` to add to each child and remove by
identity. The union's function is `&'static dyn Fn(&'static Event)`, what
JS keeps; `with_this` gave a `Box<dyn Fn(..)>`, which only `Box::leak`
made one of: a leak written to fit the binding, which a port mustn't
write.

**`with_this` gives `&'l (dyn Fn(..A) -> R + 'l)`**, the JS function
itself, which JS keeps as long as what its closure keeps, as
`webapi::listener(..)`'s `&'static dyn Fn(&E)` is: the binding owns its
lifetime, as JS does. It's a listener, a property's function, a
constructor `Class::of` takes and a value `js::unknown_of` takes, each as
it is; Rust calls it as before, its `this` `None`. Its captures are never
dropped, as a function JS keeps never is: no box the program drops runs
their drops. `Parameters`' impls bound each parameter and `R` by `'l`,
which a reference to a `dyn` of them needs.

Callers migrate their types, `Box<dyn Fn(u32) -> String>` to `&'static
dyn Fn(u32) -> String` (`&dyn Fn(u32) -> bool` of one borrowing `self`).
`function::bind`'s result is still a box (decision pending: the same
reference, for one form of a JS function made).

Held by `test/js-functions.test.ts` (each with_this test, its types
migrated), `test/js-constructors.test.ts`,
`test/lowering-module-variables.test.ts` and `test/webapi-events.test.ts`'s
"a function of its this is kept as a listener" (`kept_wrapped`,
`EventListener(function::with_this(..))`, no leak). No mutation: a
binding's signature.

## Amendment: a thread-local's is declared, its cast `this` read in place (rd196)

React DOM's suspensey resources declare `function onUnsuspend() {
this.count--; .. }` and bind it to the state, `onUnsuspend.bind(state)`.
The port holds it in a thread-local, `static ON_UNSUSPEND: &'static dyn
Fn() = function::with_this(|this, ()| { let state: &SuspendedState =
unsafe { js::cast(this) }; .. })`, which printed `const onUnsuspend =
function () { const state = this; state.count--; };`.

- **A module's `const` of a function made as a value is declared**,
  `function onUnsuspend() { .. }`, as a block's is (`prepare.rs`'s
  `declared_module_functions`). Its hoisting changes nothing: a read
  before its `const` threw. One its module sets (ADR 0270), a `let`,
  stays a variable.
- **A variable that only holds the function's `this`**, a `const` of it
  at its body's top, cast to what's bound, **is `this` where it's read**,
  as the closure's `this` is. One a function made in it reads,
  `function inner() { return state.count; }`, whose `this` is its own,
  keeps its `const`. A body lowered names each variable apart, a shadow
  too, so each read of its name is of it.
- **`this` is a path, as a variable is** (`js::same_path`), so
  `this.count = this.count - 1` is `this.count--`, a class's methods'
  too.

Case: none ([0262](0262-when-rust-and-js-disagree.md)'s first): the same
function, its name and its `this`. Held by `test/js-functions.test.ts`'s
"a with_this thread-local is a function declaration, its cast this read
in place", which calls each, and the guide's suspensey resources
(`interop/async.md`); mutations `module-function-const`,
`module-function-mutable-declared`, `this-alias-kept`,
`this-alias-nested-read` and `this-no-path`.

## Amendment: as many parameters as a function, sixteen (rd400)

React's Flight client's `ResponseInstance` is a constructor function of
14 parameters, createResponse's, `Class::of(function::with_this(|this,
(bundler_config, .., debug_end_time)| ..))`, which rustc refused: a tuple
of `Parameters`, and a `Class`'s `Construct`, `Method` and `Static`, were
each of up to 12, where a function's own `AnyFunction`, its rest
arguments and `bind` took 16, each its own list.

- **One list, `arities!` in the js crate, of each tuple of parameters
  from `()` to sixteen**, is what each of them is implemented for:
  `with_this`'s `Parameters`, `Class::of`'s `Signature` and `new`'s
  `Construct`, `js::method`'s `Method`, `call_static`'s `Static`, a rest
  parameter's tuple and `AnyFunction`. `bind`'s `Bound`, which pairs each
  parameter with what it keeps, keeps its own list of the same sixteen.
  A function takes as many through each, so a larger one is one list's
  change, never a second form.
- Nothing in rust-js counts them: lowering makes a tuple of any length a
  function's parameters, `function ResponseInstance(a, .., n)`.

Case: none ([0262](0262-when-rust-and-js-disagree.md)'s first): a
declaration's limit, rustc's to check. Held by
`test/js-constructors.test.ts`'s "a constructor function takes as many
parameters as a function", which constructs one of 14 from Rust and from
JS. No mutation: the change is the js crate's declarations, which rustc
checks, not a compiler source's.

## Amendment: a place of its `this` is never read in a handle's accessors (#449)

A P0, found by React's Flight client's `ResponseInstance`: a write
through a `RefCell` field of the constructor's `this`, in a block,

```rust
if __DEV__ {
    *this._weak_response.borrow_mut() = Some(Rc::new(WeakResponse { .. }));
}
```

printed `({ get value() { return this._weakResponse; }, set value(value)
{ this._weakResponse = value; } }).value = { .. };`, the place's handle,
whose getter and setter read `this` as their own, the handle object: the
response never had `_weakResponse`, and 82 of the client's 95 tests
failed. A struct whose fields a getter may answer, a JS object cast to
it, kept the handle, as a place that may not read the same twice can't
be its own: tidy's `handles_in_place` took one of a variable alone.

Two changes, each by its owner:

- **A handle made to read or write its place there is the place** where
  that place is a property of what reads the same, a getter or a setter
  of it called by either alike, its object read first either way
  (`its_place`, `tidy/values.rs`). So the write is
  `this._weakResponse = { .. };`, as upstream writes it.
- **A handle kept on a place of `this`**, one lent, `bump(&mut
  this.count.borrow_mut())`, reads `this` through `const this$ = this;`
  before its getter and setter, as a function nested in a `with_this`
  function reads it: `js::this_outside_accessors`, one rule, which
  0761's class members (#359) name after their class, `const counted =
  this;`, and preparation (`prepare/this_handles.rs`) applies to every
  other function's body. A function nested in it, whose `this` is its
  own, is left as it is.

Case: results changed (0262's third): Rust's, the instance's field set,
is kept. Held by `test/js-constructors.test.ts`'s "a constructor's write
through its this's RefCell field sets the instance's", run in Node; a
sweep of the corpus's, the guide's and the React ports' JS found no
other accessor reading `this`. Mutations `handle-of-a-getters-object-kept` (`tidy/values.rs`),
`this-handles-not-read-apart` (`prepare.rs`) and #359's
`class-handle-this-unnamed`, now `js/stmt_walk.rs`'s.


Amended, 2026-11-02 (#459): the rule covers a trait's default too.
1035's default copied into a class's prototype, `Keys.prototype.step`,
keeps a handle on `this.count` until tidy's `drops` stage. Before
3ce9fbb21 that stage's JS read `this.count` in the handle's own getter and
setter, and `class_trait_methods` printed `5 5` where Rust prints `6 9`.
The nightly's every-case tidy-stages run found it each night from
78f195185 on. `prepare`'s `this_read_apart` is applied to every function
body, so the default reads `const this$ = this;` as well. The case is now
one of the few cases `test/tidy-stages.test.ts` runs in the nightly's suite, and
`this-handles-not-read-apart` names it too: without the rule, and with
3ce9fbb21's parent, it fails after `flags/settled flags`.
