# 0907. A fn pointer runs what the crate made it of

Status: Accepted. Extends [0580](0580-a-call-asks-of-a-cell-only-what-it-may-borrow.md)
(what a call may borrow) to a call of a fn pointer, which ran "what
rust-js can't see". Amended: a fn pointer type of a `dyn` from outside
leaves the other types as they are, below; and (2026-10-27, the user's
decision) `function::bind` binds a function's first arguments too, its
`this` a binding's function itself, below.

Case ([0262](0262-when-rust-and-js-disagree.md)): A. A borrow left
uncounted is one no other borrow of its cell can meet: a fn pointer only
ever holds a function the crate's code made one of, each of which is seen
to borrow nothing that may be the cell held. Where a value of its type may
come from outside the crate's code, it's anything, as before, and its
borrows are counted and panic where Rust's do.

## Context

React's ReactFiberLane walks a root's lanes while it holds its tables:

```js
export function getEntangledLanes(root, renderLanes) {
  ..
    const entanglements = root.entanglements;
    let lanes = entangledLanes & allEntangledLanes;
    while (lanes > 0) {
      const index = pickArbitraryLaneIndex(lanes);
      ..
      entangledLanes |= entanglements[index];
```

and `pickArbitraryLaneIndex` counts leading zeros through `clz32`, which
is `Math.clz32` where the host has it and React's fallback where it
doesn't. Its port is a thread-local fn pointer:

```rust
thread_local! {
    pub static CLZ32: fn(u32) -> u32 = if js::is_defined(u32::leading_zeros) {
        u32::leading_zeros
    } else {
        clz32_fallback
    };
}

fn pick_arbitrary_lane_index(lanes: Lanes) -> usize {
    31 - CLZ32.with(|clz32| clz32(lanes as u32)) as usize
}
```

A call of a fn pointer was one of code rust-js can't see (ADR 0580), so
`pick_arbitrary_lane_index` might borrow anything, and every borrow held
across it was counted: `$borrow(root.entanglements, true)` and a `try {
.. } finally { $unborrow(..) }` in `getEntangledLanes`, `$borrowMut(..)`
in `markStarvedLanesAsExpired`, and in `markRootFinished`, which holds
three, a `try`/`catch` around each call that undid each borrow live, and
its `for` a `$iter`/`$next` loop, each step of which may throw.

Counting is by type (ADR 0863): once one borrow of a `RefCell<Vec<i32>>`
is held while code that may borrow one runs, every borrow of that type
is checked, since the code run may be any of them. So the one fn pointer
counted every `LaneMap` of the root's: `entanglements` of `Lanes`,
`expirationTimes` of `f64`, `hiddenUpdates`, `pendingUpdatersLaneMap` and
`transitionLanes` too.

## Decision

**A fn pointer's call runs what the pointer may be** (`analysis/fn_pointers.rs`,
`FnPointers::may_be`): each function and closure the crate's code makes a
fn pointer of that type of, by a coercion anywhere in its bodies, which
rustc's typeck records, the initializers of its statics and thread-locals
among them; or anything, where a value of its type may come from outside
the crate's code. What it may be is asked as a call of it is: a function
of the crate's runs what its summary says, std's what its bounds do, a
binding what its marks say (ADRs 0867, 0898), and a closure its body.
Both the crate's summaries (`direct`) and a guard's extent (`Quiet`, ADR
0328) ask it, through `CellBorrows::pointer_call_asks`.

A fn pointer is made only by a coercion, of a function or of a closure
that captures nothing. One the crate didn't make reaches its code from
outside it only by what crosses into it, so a type is *anything* where a
value that may hold one of its fn pointers, through what it's made of and
the fields of a type that isn't std's:

- is given to, or by, a library's function or a binding, which may make
  one of any function, give one, or change one it's given what holds;
- is a library's static or constant, or a library's `Deref` gives it;
- is a parameter or the result of a function JS or a consumer may call,
  an exported one or a trait impl's (`library::reachable`);
- is what std gives of what it isn't given, by addressing memory or a
  downcast of a `dyn Any` (`recognition::gives_what_it_isnt_given`): any
  other std function gives only what it's given, and an empty `Vec<T>`
  holds no `T`;
- is the result of a generic function of the crate's whose signature's
  result names a type parameter, which may be one it has from outside.

Its type is one whatever its lifetimes: `for<'a> fn(&'a Root)` and
`fn(&'static Root)` are one. One whose signature holds a fn pointer or a
`dyn` of its own, whose binders aren't compared, is anything.

```rust
pub fn entangled_lanes(root: &Root, mut lanes: i32) -> i32 {
    let entanglements = root.entanglements.borrow();
    let mut entangled = lanes;
    while lanes > 0 {
        let index = pick_arbitrary_lane_index(lanes);
        entangled |= entanglements[index];
        lanes &= !(1 << index);
    }
    entangled
}
```

```js
export function entangledLanes(root, lanes) {
  const entanglements = root.entanglements;
  let entangled = lanes;
  while (lanes > 0) {
    const index = pickArbitraryLaneIndex(lanes);
    entangled |= $index(entanglements, index);
    lanes &= ~(1 << index);
  }
  return entangled;
}
```

## Why

- **The check it drops can't fail**: what the call runs is one of the
  functions the crate made the pointer of, each seen to borrow nothing
  that may be the cell held.
- **Rust's own closed world**: a fn pointer is a function some coercion
  made one of. The crate's are all in its bodies; the others come in only
  through what crosses its edge, which the types say.
- **Upstream's JS**: `const entanglements = root.entanglements`, a table
  read with no count kept of a borrow no other can meet.

## Alternatives

- **Follow the value**: which fn pointer a call reads, `CLZ32`'s, and
  what its initializer may give. Exact here, but a flow analysis of every
  place a fn pointer passes through, where a type answers each the same.
  Rejected.
- **Count per borrow, not per type**: only a borrow held while code that
  may borrow its cell runs keeps a count; every other one of the type
  still asks whether one is held, since the code run while that one is
  held may be any of them. So the others would still be a check each,
  `$borrow(..)`'s or another of its own. The type's count stays (ADR
  0863); this decision is what took the `LaneMap`s out, by the fn pointer
  no longer running anything.
- **Trust every fn pointer to run only the crate's functions**: a JS
  function JS gives an exported function, which borrows the cell held,
  would pass where Rust's panics. Rejected.

## Consequences

- No corpus case's JS changes but the new ones. React's ReactFiberLane
  has no `$borrow`, `$borrowMut`, `$unborrow`, `$iter` or `$next` left.
- `docs/readability.txt` is blessed with its counts as they were: the
  corpus cases this change and its branch's add (`fn_pointer_*`,
  `integer_truth`, `index_compound_assign`), 280 lines with none of its
  shapes, make each shape's share of 10,000 lines smaller, `$iter` 87.8
  to 87.6, `renamed x$1` 734.8 to 732.9. `$iter`'s count, 919, is
  `main`'s, grown from 903 by its own FlatMap and `for` changes.
- A fn pointer of a type JS or a library may give, of an exported
  function's parameter say, is anything: a borrow held across a call of
  one is counted, of every `RefCell` of its type.
- A type's fn pointers are each the crate made one of anywhere, not only
  those that reach the call: a pointer of the same type that does borrow
  the cell, made elsewhere, counts it.

## What holds it

- `test/lowering-cell-borrows.test.ts`, "a borrow held while a fn pointer
  runs what the crate made it of isn't counted": `clz32` of
  `Math.clz32` or its fallback, its table the array; and a fn pointer JS
  gives an exported function, counted, which throws `already borrowed`
  where it borrows the cell.
- The corpus's `fn_pointer_borrows_nothing`, `fn_pointer_borrows_held`
  and `fn_pointer_closure_borrows_held`, against native Rust: a fn
  pointer of a function or a closure that borrows the cell held panics.
- Mutations in `scripts/mutations/lower/analysis/fn_pointers.ts`:
  `fn-pointer-made-of-nothing`, `fn-pointer-closures-made-of-nothing`,
  `fn-pointer-never-from-outside`; in `cell_borrows.ts`,
  `fn-pointer-call-runs-anything`; in `drops/facts.ts`,
  `pointer-call-beside-guard-asks-nothing`.

## Amendment: a type of a `dyn` from outside leaves the others as they are

Case ([0262](0262-when-rust-and-js-disagree.md)): A, as before. React's
ReactFiberCallUserSpace binds each DEV frame's function, `const
callComponentInDEV = callComponent.react_stack_bottom_frame.bind(callComponent)`,
whose port is `function::bind(CALL_ADD.react_stack_bottom_frame,
Some(&CALL_ADD))` of a `fn(&dyn Fn() -> u32) -> u32`. A binding's result
comes from outside, and a fn pointer type whose signature holds a `dyn`
or a fn pointer has no key, its binders not compared; held, such a type
made *every* fn pointer type anything. So ReactFiberLane's `clz32`, a
`fn(u32) -> u32` the crate made of `Math.clz32` alone, ran anything
again, and each borrow held across it was `$borrowMut` and a `try`/`finally`.

**A fn pointer type of no key, given from outside, is anything itself,
as before, and leaves every other type as it is**: no coercion or
subtyping makes a value of one into a value of a type that has a key,
whose signature holds neither a `dyn` nor a fn pointer. What its
signature holds may come from outside with it, a `fn() -> fn(u8) -> u8`'s
`fn(u8) -> u8`, which the walk of what a type holds reaches anyway.
What still makes every type anything is a type rustc can't normalize.

- `test/lowering-cell-borrows.test.ts`, "a fn pointer of a dyn given
  from outside leaves other fn pointers what the crate made them of":
  `clz32` under a borrow uncounted beside the bound frame, the frame's
  call counted, and a fn pointer a JS function returns counted, which
  throws `already borrowed` where it borrows the cell.
- Mutation `fn-pointer-of-a-dyn-opens-all`.

## Amendment (2026-10-27): `bind` binds a function's first arguments

The user's decision: `js::function::bind` gains a rest of leading
arguments, so a port writes upstream's `.bind(thisArg, ...args)`.
React's hooks bind each dispatch to its fiber and queue,

```js
const dispatch = dispatchSetState.bind(null, currentlyRenderingFiber, queue);
```

and its performance track logs each phase so, about 23 times:

```js
debugTask.run(console.timeStamp.bind(console, label, start, end, track, group, color));
```

ADR 0740's amendment left the arguments after `thisArg` out: a function
of fewer parameters is a type Rust can't name of `F`'s, and a closure,
`move |action| dispatch_set_state(&fiber, &queue, action)`, was the way.
Its JS is an arrow, where upstream's is the bound function.

**`function::bind(f, this_arg, args)` is `f.bind(thisArg, ...args)`**:
`args`, the tuple of `f`'s first arguments, is a `#[rust_js::variadic]`
binding's rest (ADR 0640), `()` of none, spread. What it gives is typed
as TypeScript types it:

- **Of none, `()`, a function of `f`'s own type**, `F: Copy`, as ADR
  0740 amended had it: `OmitThisParameter<T>`. A `fn` pointer stays one,
  each of its parameters' lifetimes its own (`for<'a> fn(&'a Fiber)`), so
  React's frames are `fn`s as before.
- **Of some, a `Box<dyn Fn(..) -> R + 'l>` of the parameters `f` has
  left**, which lives as long as what it keeps, `'l`. The trait
  `function::Bound<'l, L>`, of `(F, M)`, `M` the `fn` pointer of `f`'s
  signature (`AnyFunction`'s marker), names it: one impl for each
  signature of up to 12 parameters and each count bound. rustc checks
  each argument against its parameter, and how long the bound function
  may live.
- **Each argument is kept** (`function::Keeps<P>`): a value of a `Copy`
  type, given to each call as it is, or what a parameter borrows, `Rc<Fiber>`
  of a `&Rc<Fiber>`, lent to each call, as a closure `move |action|
  f(&fiber, action)` lends what it keeps. A value moved into a parameter
  by value isn't kept: each call would move it again, where JS gives each
  the one value. In JS each is the value itself; a counted `Rc` given to
  a binding stays refused (ADR 0850).
- **A shared one is `Rc::from(boxed)`**, the box's value (ADR 0023's
  `Box`, `rc_made_of`), React's `queue.dispatch` and the hook's result
  one function.
- **A binding's function given as the value it is**, to a parameter of
  `F: AnyFunction<M>` or `AnyShape<M>`, `bind`'s, `name`'s, `unknown`'s,
  **is its JS function itself** where its arrow only passes its
  parameters on, as a `fn` of it is (ADR 0602): `console.timeStamp.bind(..)`,
  not `((label, ..) => console.timeStamp(label, ..)).bind(..)`, whose
  `name` was `""` where `console.timeStamp`'s is `"timeStamp"`, a wrong
  answer of `function::name(&console::time_stamp)` before. A closure the
  program writes stays the function it is; a callback JS calls,
  `then(parseFloat)`'s, stays the arrow of what Rust gives
  (`binding_calls.rs`'s `functions_themselves`).
- **What the bound function runs**: a binding's result, from outside, so
  a call of it may run anything, as before; a `Box<dyn Fn>` is called as
  a `dyn` is.

```rust
pub fn mount_state(fiber: Rc<Fiber>, queue: Rc<UpdateQueue>) -> Rc<Dispatch> {
    Rc::from(function::bind(dispatch_set_state, Some(&Kind::Null), (fiber, queue)))
}

// What's given an `Option<&dyn IntoStrOrNumber>` parameter is one: no
// coercion reaches into a tuple a generic function takes.
let (start, end): (&dyn IntoStrOrNumber, &dyn IntoStrOrNumber) = (&start, &end);
debug_task.run(function::bind(
    console::time_stamp,
    Some(&console),
    (label, Some(start), Some(end), Some(COMPONENTS_TRACK), None, Some(color)),
));
```

```js
export function mountState(fiber, queue) {
  return dispatchSetState.bind(null, fiber, queue);
}

debugTask.run(console.timeStamp.bind(console, label, start, end, COMPONENTS_TRACK, undefined, color));
```

Case N ([0262](0262-when-rust-and-js-disagree.md)): the binding is JS's
own `bind`, and what each call is given is what Rust gives it.

### Alternatives

- **Every bound function a `Box<dyn Fn>`**, of none bound too: a
  remaining parameter that borrows has one lifetime, not `for<'a>`, so a
  frame's `fn(&Fiber, u32)` bound couldn't be the `fn` it's stored as, and
  a `Box` isn't `Copy`, `.with(|call| *call)`. Of none bound, `F` itself
  is TypeScript's answer too.
- **A `fn` pointer of the parameters left**: it would outlive what it
  borrows, `Some(&start)`, which rustc must check; and keep a borrow only
  where it's `'static`.
- **The caller names the bound function's type**, `bind::<.., fn(u32)>`:
  rustc couldn't check it against `f`'s.
- **`Rc<dyn Fn>`**: shared without `Rc::from`, but `run(&*..)` where a
  callback is an `impl FnOnce`.

### Limits

- A parameter left that borrows, `fn(&str)` of `f(fiber, label: &str)`
  with `fiber` bound, has the one lifetime rustc infers, not `for<'a>`; a
  closure is the way where it must be any.
- Up to 16 parameters, as `AnyFunction`'s (12 before the amendment
  below).

Held by `test/js-object.test.ts`, "a function bound to its first
arguments is a function of the parameters left" (dispatch's `Rc`s lent,
numbers kept, `console.timeStamp` bound to `console` and its `name`, a
closure's `name`, `then`'s callback an arrow, run) and "a function bound
to a this is a function of its signature"; mutations
`binding-function-value-an-arrow`, `any-function-value-itself`,
`callback-binding-function-itself` (`binding_calls.ts`) and
`rc-from-box-unsupported` (`recognition/wrappers.ts`).

## Amendment: a binding's method bound is the method, its trailing `None` left out where each parameter is

React's ReactFiberPerformanceTrack writes
`debugTask.run(performance.measure.bind(performance, measureName,
options))`. Its port's `function::bind(Performance::measure,
Some(&performance), (performance, name, Some(options), None))` printed an
arrow of the method bound, `((self, measureName, ..) =>
self.measure(..)).bind(performance, performance, measureName, options,
undefined)`: a binding's method as a value is its arrow of its receiver,
and the bound `None` was given to JS.

**`function::bind` of a binding's method, its receiver the first argument
bound and its `this` the same path, is the method of that receiver
bound**, `performance.measure.bind(performance, name, options)`: what the
arrow does, as `this` is the receiver. **And where each of the bound
function's parameters is bound, a trailing `None` of an optional one,
TypeScript's `x?`, is left out**, as its call leaves it (ADR 0443). Where
parameters are left to the call, a bound `None` stays, `undefined`: left
out, what the call gives next would be taken for it. Case N: the same
calls with the same values (a WebIDL operation takes `undefined` as
missing).

Held by `test/js-object.test.ts`'s "a function bound to its first
arguments is a function of the parameters left": `performance.measure`
bound and run, and `console.timeStamp.bind(console, label, undefined)` of
one bound in part; the guide's `log_measure`; mutations
`bound-method-an-arrow`, `bound-none-left-out-of-a-part` and
`bound-none-kept`.


## Amendment: sixteen parameters, and an `Option` of what a parameter borrows

React's ReactFiberWorkLoop binds fourteen and fifteen arguments,
`completeRootWhenReady.bind(null, root, finishedWork,
workInProgressRootRecoverableErrors, ..)` and `completeRoot.bind(null,
root, finishedWork, lanes, recoverableErrors, ..)`, and several of them
are nullable, `recoverableErrors: null | Array<CapturedValue<mixed>>`,
`finishedWork: null | Fiber`, whose parameters are `Option<&T>`. Its
port's `function::bind(complete_root, Some(&Kind::Null), (Rc::clone(root),
errors, ..))` was refused twice: past twelve, and "a bound function can't
keep a `Option<Rc<Errors>>` for a parameter of `Option<&Rc<Errors>>`". The
port wrote a closure, `Box::new(move || complete_root(&root, errors.as_ref(),
..))`, an arrow where upstream's is the bound function (the port's gap wl3).

**`AnyFunction`, `AnyShape`, a rest's tuple and `Bound` are of up to 16
parameters**, upstream's fifteen and one more; **and a bound function keeps
an `Option<T>` for a parameter of `Option<&T>`**, lent to each call as
`kept.as_ref()`, as it keeps a `T` for a `&T`. In JS each is the value
itself: a reference is what it borrows, so an `Option<&T>` is what an
`Option<T>` is, `null`-or-`undefined` of `None`, an `Option` of an
`Option` the same box of each. The impl is one more of `Keeps`, the one
rule of what a bound function keeps; it overlaps neither other, as an
`Option<T>` is never an `Option<&T>` of itself. `with_this`'s
`Parameters` stays at 12: no upstream function reads its `this` of more.

```rust
function::bind(
    complete_root,
    Some(&Kind::Null),
    (Rc::clone(root), finished_work, lanes, recoverable_errors, transitions, ..),
)
```

```js
completeRoot.bind(null, root, finishedWork, lanes, recoverableErrors, transitions, ..)
```

Case N ([0262](0262-when-rust-and-js-disagree.md)): the binding is JS's
own `bind`, and each call is given what Rust lends it.

Held by `test/js-functions.test.ts`'s "a function bound to fifteen
arguments, each an Option of what its parameter borrows": an
`Option<Rc<..>>`, an `Option<Option<u32>>`, an `Option<()>` and an
`Option<String>` kept, each `Some` and `None`, the bound function called
twice. Each is a type of `builtins/src/lib.rs`, which no mutation list
covers (mutations are the compiler's, ADR 0093): taking the impl or an
arity out fails that test's compile.

Not covered: a parameter left that borrows still has one lifetime (Limits,
above), React's `callback.bind(null, instance)` of a view transition's
`callback(instance, types)` (the port's gap wl1).

## Amendment: a signature that holds a `dyn` is keyed (rd56)

Every fn pointer type is keyed, its lifetimes erased, those its signature
binds too, so one whose signature holds a `dyn Fn`, React's
`Children.forEach`'s, is known as any other is; and a consumer reads a
library's field its contract says holds only Rust functions as such. See
[1007](1007-a-libraries-own-fn-pointers-are-rust-functions.md).

## Amendment: a function bound to arguments is the JS function itself, `&'l dyn Fn(..)` (rd209)

`function::with_this` gives the JS function itself, `&'l dyn Fn(..)`
(ADR 0972, amended), which JS keeps. `bind` of some arguments gave a
`Box<dyn Fn(..)>`: two forms of a function JS makes. **It gives `&'l (dyn
Fn(..) -> R + 'l)` too**, the parameters left and `R` bound by `'l`; of
none, `F` itself as before. What it keeps is never dropped, as JS keeps
the function. `Rc::from(function::bind(..))` still shares it, an
`Rc<&dyn Fn>` unsized to `Rc<dyn Fn>`; a program holds the reference
itself where nothing else needs an `Rc`. Held by `test/js-functions.test.ts`'s
"a function bound to fifteen arguments" (its return type migrated) and
`test/js-object.test.ts`'s bind tests. No mutation: a binding's signature.
