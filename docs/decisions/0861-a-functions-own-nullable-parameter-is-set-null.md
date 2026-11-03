# 0861. A function's own nullable parameter is `null` where it's given or set `None`

Status: Accepted. Amended (2026-10-29): its map by what's its argument in JS is stored as given. Amended (2026-11-02): a parameter each call gives `null` or a value is tested `!== null` (rd613). Extends [0275](0275-nullable-writes-null.md)'s
`#[rust_js::nullable(name)]`, a binding's parameter, to a function of the
crate's own, and [0772](0772-a-nullable-cell-field-writes-null.md) and
[0841](0841-a-nullable-place-stores-what-is-never-undefined-as-it-is.md)'s
nullable places to it. Amended (2026-10-28): a closure's parameter names
its own nullable bindings.

Case ([0262](0262-when-rust-and-js-disagree.md)): A, decided by the user
(2026-10-25): an `Option<&Unknown>`'s `None` given to a place typed `T |
null` is JS's `null`, elsewhere `undefined`. The program reads `null` and
`undefined` alike as `None` (ADR 0030), and an any-shape value's `None` is
either already (ADR 0676); only what JS is given changes.

## Context

React's `mapIntoArray` (ReactChildren.js) makes `undefined` and a boolean
`null`, and calls the callback with it:

```js
if (type === 'undefined' || type === 'boolean') {
  // All of the above are perceived as null.
  children = null;
}
..
let mappedChild = callback(child);
```

Its tests say so, `expect(callback).toHaveBeenCalledWith(null, 5)` of a
child that was `true`, `false`, `null` or `undefined`. The port's
`children = None;` was `children = undefined;`, as every `None` is. A
`#[rust_js::nullable(children)]` on the function was taken, and its call's
`map_into_array(None, ..)` was `null` already (ADR 0781's `null_nones`,
which asks any function's), but not where the body set it.

## Decision

**`#[rust_js::nullable(name)]` on a function of the crate names a
parameter of its own, `T | null` as a nullable field is: a `None` written
to it is `null`, where a call gives it and where its body sets it.** The
body sets it as it sets a nullable field, `Storage::NullForNone`'s: a
`None` `null`, a `Some(..)` or what's `null` or a value already as it is,
an `Option` that may be `undefined` `?? null`.

```rust
#[cfg_attr(rust_js, rust_js::nullable(children))]
fn map_into_array(mut children: Option<&'static Unknown>, ..) -> u32 {
    let r#type = js::type_of(children);
    if r#type == "undefined" || r#type == "boolean" {
        // All of the above are perceived as null.
        children = None;
    }
    ..
}
```

```js
function mapIntoArray(children, ..) {
  const type = typeof children;
  if (type === "undefined" || type === "boolean") {
    // All of the above are perceived as null.
    children = null;
  }
  ..
}
```

- **What a call gives it is as it is**, a variable, `mapIntoArray(child,
  ..)`, or a call's `Option`, `mapIntoArray(init(payload), ..)`; a `None`
  written there is `null`, `mapIntoArray(null, ..)`: the function's own
  body reads it, which reads `null` and `undefined` alike, where a
  binding's JS may not, whose argument is `child ?? null` (ADR 0275), and
  a call's `Option` only it reads `f() ?? null` (ADR 0841,
  `mir_null_nones`, which now tells what it's stored in apart).
- **Its `.d.ts` is `T | null | undefined`**, as a nullable field's is
  `?: T | null`: a JS caller may give either.
- **One attribute, one meaning**: the parameter `T | null`, of a binding
  and of the crate's function alike; on the function, as a binding's is,
  since a parameter's own attributes aren't kept in a crate's metadata.

## Why

- **React's JS**, and what its tests see: the callback given `null`.
- **The decision the user made**: a nullable place's `None` is `null`.

## Alternatives

- **Coalesce what each call gives**, `mapIntoArray(init(payload) ?? null,
  ..)`: six `?? null` in React's function where upstream has none, of
  what its body reads either way.
- **`null` of every `Option<&Unknown>`'s `None`**: what isn't typed `T |
  null` stays `undefined`, as the user decided.

## What holds it

- `test/lowering-nullable.test.ts`, "a function's own nullable parameter
  set to None is null": `undefined`, `true`, `null` and a number given, a
  `None` given, and a parameter not marked beside it, each callback's
  argument checked.
- Mutations `own-nullable-param-set-undefined`,
  `own-nullable-param-any-argument` (`scripts/mutations/lower/mir/assign.ts`)
  and `null-nones-own-param-call-coalesced` (`lower/mir/patterns.ts`).

## Amendment: a nullable parameter is `T | null` as declared, and what only nulls is returned as it is

The user's decision (2026-10-27), Case A ([0262](0262-when-rust-and-js-disagree.md)):
React's ports printed `?? null` where upstream passes a value on:

| Rust | Was | Upstream |
|---|---|---|
| `capture_commit_phase_error(current, nearest_mounted_ancestor)`, both `#[rust_js::nullable(..)]` | `captureCommitPhaseError(current, nearestMountedAncestor ?? null)` | `captureCommitPhaseError(current, nearestMountedAncestor)` |
| `SEARCH_TARGET.set(target)` of a nullable parameter | `searchTarget = target ?? null` | `searchTarget = target` |
| `SEARCH_TARGET.with_borrow(Clone::clone)` | `return searchTarget ?? null` | `return searchTarget` |
| `let mut root = None; .. root = Some(Rc::clone(&n)); .. root` | `let root; .. return root ?? null` | `let root = null; .. return root` |

A nullable parameter's `None` was `null` where the crate's calls give it,
but its `.d.ts` said `T | null | undefined`, so a JS caller might give
`undefined`, which `?? null` turned to `null` wherever it went next.

**A `#[rust_js::nullable(..)]` parameter of the crate's function is `T |
null` as declared**: its `.d.ts` is `fiber: Fiber | null`, never left
out, and its body takes it as `null` or a value already
(`stored_given`'s `Nulled`), given on to a nullable parameter or stored
in a nullable place as it is. A JS caller that gives `undefined` breaks
the declaration, as it would TypeScript's. The body still reads `null`
and `undefined` alike as `None` (ADR 0030).

And what a nullable function returns that holds only `null`s and values:

- **a nullable thread-local's whole value cloned**,
  `KEY.with_borrow(Clone::clone)`: every write of it stores `null` for
  `None` (ADR 0586, amended), and ES modules give importers no way to set
  a module's variable, so it's `null` or a value (`QuietCells::nullable`,
  `Known::Defined`);
- **a `let` of `None` the function returns**, set only to `Some(..)`s,
  one MIR makes in a temporary and moves in too: the return is a
  nullable place it's read by (`null_nones`, ADR 0781), so its `None` is
  `null` where it's made, `let root = null;`, and it's returned as it is
  (ADR 0957's let set only to values, extended to one that starts
  `None`).

A nullable field's read kept in a `let` and returned keeps its `?? null`:
JS may have set the field `undefined`.

Held by `test/lowering-nullable-reads.test.ts`'s "a nullable parameter,
a nullable thread-local's clone and a let of None returned are as they
are" and `test/lowering-module-variables.test.ts`'s "a nullable
thread-local's None is null where it's made and set", its `.d.ts`
`setCurrentFiber(fiber: Fiber | null)`, run in JS; and the mutations
`own-nullable-param-maybe-undefined` (`mir/rvalues.ts`),
`nullable-local-clone-unread` (`mir/local_facts.ts`),
`nullable-local-clone-maybe-undefined` (`std_types/cell.ts`),
`nullable-return-not-a-nullable-read`, `some-in-a-temporary-not-a-write`
(`mir/patterns.ts`) and `nullable-param-declared-optional`
(`declarations/functions.ts`).


## Amendment: a closure's parameter names its own nullable bindings (2026-10-28)

Case ([0262](0262-when-rust-and-js-disagree.md)): A, the user's decision
above, extended: a binding declared `T | null` is taken as `null` or a
value, as a function's own parameter is.

React DOM's `SyntheticBaseEvent` (React port gap rd64d) is a
`function::with_this` closure (ADR 1009), its parameters a tuple:

```js
function SyntheticBaseEvent(reactName, reactEventType, targetInst, nativeEvent, nativeEventTarget) {
  this._reactName = reactName;
  this._targetInst = targetInst;
  ..
  this.target = nativeEventTarget;
```

Upstream types `reactName: string | null`, `targetInst: Fiber | null`
and `nativeEventTarget: null | EventTarget`. Each stored in a nullable
field was `reactName ?? null`: a closure takes no attribute (an
attribute on an expression is unstable), so nothing could say so, and a
closure's parameter may be what JS gave.

**A closure's parameter takes `#[rust_js::nullable(..)]`, naming the
bindings of its pattern that are `T | null`**, as a function's own does
on the function: each is `null` or a value already, stored in a nullable
place as it is, and `null` where the body sets it `None`.

```rust
let synthetic_base_event = Class::of(function::with_this(
    move |this,
          #[cfg_attr(rust_js, rust_js::nullable(react_name, target_inst, native_event_target))] (
        react_name,
        react_event_type,
        target_inst,
        native_event,
        native_event_target,
    ): (Option<&'static str>, &'static str, Option<&'static Fiber>, &'static Event, Option<&'static EventTarget>)| {
        ..
        this.react_name.set(react_name);
```

```js
this._reactName = reactName;
```

- **On the parameter, of a closure only**: a closure's parameter is the
  one place an attribute can say it, and no other crate calls a closure
  by its name, whose metadata would lose it. A function's own parameter
  says so on the function, as a binding's does; on its parameter it's an
  error naming that form, `a function's parameter is
  #[rust_js::nullable(value)] on the function`, so there's one place for
  each.
- **Each name is a binding of that parameter that's an `Option`**, else
  an error (`nullable_parameters_refused`); read by the binding's debug
  info, its variable or the tuple's item (`nullable_bindings`,
  `nullable_binding`).
- **What gives it `undefined` breaks the declaration**, as a JS caller of
  a function's own does (above). A Rust caller constructs it through the
  class it's held as, `js::new(class, (None, ..))`, whose `None` is
  `undefined`, as `js::new` can't know which function the class holds:
  where upstream passes `null`, the port writes `js::null()`, `new
  SyntheticEventCtor(reactName, reactEventType, null, ..)`, which is
  upstream's JS too. An unnamed binding is `?? null` still.

- **Its bindings are the function's parameters** (rd91): MIR makes each
  of the tuple's item, `name = (_3.0)`, which is the binding's own value,
  not a `None` the body sets (`nullable_tuple_item`), so `function
  Made(name, inst)`, where the port's shape, a `move` closure in its
  factory, was `function Made(_3, _3$2) { const _3$1 = [_3, _3$2]; const
  name = _3$1[0] ?? null; ..`: the same values, but MIR's names and a
  tuple made again.
- **Stored through a borrow as it is** (rd91): `*this.target_inst.
  borrow_mut() = target_inst` of a nullable `RefCell<Option<Rc<Fiber>>>`
  field writes through a handle that stores as the field does, which
  tidy applies with no word from MIR (ADR 0791); what's `null` or a value
  already, such a binding or a function's own nullable parameter, says so
  (`Known::NoneNull` on what's stored through a reference), `this.inst =
  inst;`, as a `Cell`'s `set` of it is. A function's own parameter was
  `made.inst = inst ?? null` there too.

Held by `test/js-constructors.test.ts`'s "a with_this closure's nullable
parameters are stored as they're given", "a with_this closure's nullable
bindings are its parameters, stored through a borrow as given" (named ones stored as given, one
set `None` `null`, one not named `?? null`, run in JS) and "a closure's
nullable parameter names its own Option bindings" (the two errors);
mutations `closure-nullable-binding-coalesced`,
`closure-nullable-binding-any-name`, `closure-nullable-tuple-item-coalesced`,
`borrowed-nullable-store-coalesced` (`mir/assign.ts`),
`closure-nullable-not-an-option-allowed` and `fn-param-nullable-allowed`
(`bindings/nullable.ts`).

## Amendment: its map by what's its argument is as it's given (2026-10-29)

React's SyntheticBaseEvent stores its target as any value,
`this.target.set(native_event_target.map(js::unknown))`, of a nullable
`Option<&EventTarget>` binding: `this.target = nativeEventTarget ?? null;`
where upstream writes `this.target = nativeEventTarget;` (the port's gap
rd94). `js::unknown` is its argument in JS, so the map is the option itself,
`null` or a value already.

**An `Option::map` that's the option itself in JS is what it maps, to what a
nullable place stores**: by a function each call of which is what it's given
(ADRs 0264, 0584), or by a binding that's its argument, `js::unknown`'s
(`JsForm::This`), neither payload boxed. One question answers both, the
map's lowering and what's stored as given (`std_types/option.rs`'s
`maps_to_itself`, asked by `mir/rvalues.rs`'s `stored_given`). A map by
another function is stored as its branches are, `x != null ? given(x) :
null`. Case A: the same value, `null` or the object.

- Held by `test/js-constructors.test.ts`, "an identity map of a nullable
  parameter is stored as it's given", run; mutations
  `identity-map-stored-unknown` (`lower/mir/rvalues.ts`) and
  `this-binding-map-not-itself` (`lower/std_types/option.ts`).

## Amendment: a parameter each call gives `null` or a value is tested `!== null` (2026-11-02, rd613)

The user's decision (2026-11-02, option A), Case A
([0262](0262-when-rust-and-js-disagree.md)): the same answers, of the
values its declaration and its crate's calls give.

React's Flight targets test their own nullable parameter `!== null`:

```js
export function prepareDestinationWithChunks(moduleLoading, chunks, nonce) {
  if (moduleLoading !== null) {
```

The port's `#[rust_js::nullable(module_loading)]` printed `moduleLoading
!= null`. `!== null` alone would be wrong where a call of the crate's
gives `undefined`: what a call gives is as it is (above), so
`given(module_loading)`, of a parameter JS gave `undefined`, gives it
`undefined`, which `!== null` takes for a value, `undefined.prefix`,
where Rust's `None` returns `""`.

**A function's own nullable parameter is tested `!== null` and `===
null` where each of the crate's calls of it gives it `null` or a value,
and nothing uses the function but to call it** (`analysis/nulled_params.rs`,
`NulledParams`, the owner): each call's argument as `stored_given` says
(`Nulled`: `None`, `Some(..)`, a nullable parameter or field passed on, a
nullable function's result), each call seen as lowering takes it
(`NulledCalls::record`, by the function calling and its block, so a call
lowered another way leaves it `!= null`), and the function never given or
kept as a value, nor a trait's method or a binding. JS's calls keep the
declared `T | null`, as above. The body's own writes are `null` for `None`
already, and one written through a borrow, or by a call's result in place,
leaves it `!= null` (`mir_nones`'s `fixed`). What isn't each call's stays
as it was: React's `mapIntoArray(child, ..)`, given what JS gave, keeps
`!= null`, with no `?? null` added, as the alternative "coalesce what each
call gives" (above) would.

The calls are known once every body is lowered, so each function with
such a parameter is lowered again, with its copies, its tests strict
(`pipeline.rs`). What each call gives doesn't change with it: a function's
own nullable parameter passed on is `Nulled` either way.

**Another crate's calls**: the library's contract says which parameters
(`Calling::nulled_params`, library ABI 12). A consumer's call gives what
may be `undefined` there `?? null`, and what's `null` or a value already
as it is:

```js
export function given(moduleLoading) {
  return Dep.prepare(moduleLoading ?? null, "a.js");
}
```

A consumer's use of it as a value, `let f: fn(..) -> _ = dep::prepare;`,
is an error, as a call through the value could give `undefined`: "a
function of another crate's whose nullable parameter is `null` at each of
its calls, given as a value", which says to call it in a closure.

- Held by `test/lowering-nullable-reads.test.ts`'s "a function's own
  nullable parameter each call gives null or a value is tested against
  null alone" (one called with `None`, `Some` and a nullable parameter,
  one set `None` in its body, one called with what JS gave, one mapped as
  a value, one not marked, run in JS) and `test/cross-crate-items.test.ts`'s
  "a library's nullable parameter its calls give null is given null by
  its consumer" (its contract, a consumer's `?? null` and as it is, run,
  and the refusal); mutations in `lower/analysis/nulled_params.ts`,
  `lower/mir/patterns.ts`, `lower/mir/dispatch.ts`, `lower/library.ts`
  and `lower/pipeline.ts`.
- The port marks the callers too: BundlerWebpack's
  `prepare_destination_for_module` takes `#[rust_js::nullable(module_loading)]`,
  so its call of the target's gives it on as it is.

## Amendment: a `let` of `null` set only to a tested `Some` is `null` or a value (2026-11-02, rd623)

React DOM's createRoot keeps its options' callbacks in a `let` that
starts `js::null()` and is set only under that field's own test:

```rust
let mut transition_callbacks = js::null();
if let Some(options) = options {
    if options.unstable_transition_callbacks.is_some() {
        transition_callbacks = options.unstable_transition_callbacks;
    }
}
ReactFiberReconciler.create_container(.., transition_callbacks)
```

The amendment above made the reconciler's call give it
`transitionCallbacks ?? null`, where upstream, and the port before it,
gives `transitionCallbacks`: what the `let` holds was "what may be
`undefined`", as a field's read is, and `js::null()`'s `None` a call's,
which may be `undefined` too.

**What a `let` holds follows its writes and their tests** (`mir/patterns.rs`'s
`mir_null_nones`): a call that gives `null` for `None`, `js::null()`'s
(`calls::none_given_by`), is a `None` that's `null`, and a copy of a
place read first where `place.is_some()` was just true, the block its
test alone goes on to, the place a field of what a shared reference
refers to, nothing writing between (`tested_some`), is a `Some`. So the
`let` is `null` or a value: the reconciler's call gives it as it is, and
a crate's own function given it tests it `!== null`. A copy where
another place was tested, or where it was tested `is_none()`, is what JS
gave, as before.

Case ([0262](0262-when-rust-and-js-disagree.md)): none: `?? null` of what's
never `undefined` is that value, and `!== null` of what's `null` or a
value is `!= null`.

- Held by `test/lowering-nullable-reads.test.ts`'s "a let of null set
  only to a place tested is_some is null or a value" (tested `is_some()`,
  `is_none()` and another place's test, run in JS), and the mutations
  `tested-some-copy-unseen`, `tested-some-of-any-test`,
  `tested-some-of-another-place` and `null-call-called`
  (`lower/mir/patterns.ts`).

## Amendment: a parameter given on is so only where the one it's given is (2026-11-02, rd624, #477)

#477, a wrong answer the amendment above (rd613) made. A call that gives
on the caller's own nullable parameter counted as one that gives `null`
or a value, as that parameter is `T | null` as declared:

```rust
#[cfg_attr(rust_js, rust_js::nullable(parent_component))]
fn get_context_for_subtree(parent_component: Option<&Component>) -> u32 {
    let Some(parent_component) = parent_component else { return 0; };
    parent_component.context
}

#[cfg_attr(rust_js, rust_js::nullable(parent_component))]
fn update_container_impl(element: u32, parent_component: Option<&Component>) -> u32 {
    element + get_context_for_subtree(parent_component)
}

pub fn update_container(element: u32, parent_component: Option<&Component>) -> u32 {
    update_container_impl(element, parent_component)
}
```

`update_container_impl` was tested `!= null`, one of its calls giving what
JS gave `update_container`, but `getContextForSubtree` `=== null`, so
React's `updateContainer(element, container)`, its optional arguments
left out, read `undefined._reactInternals`.

**A call that gives on the caller's own nullable parameter gives `null` or
a value only where that parameter is so itself, each of its calls giving
it so, each passing on followed to its end** (`stored_given`'s
`given_on_param`, the parameter a `Nulled` read is of; `NulledParams::record`
keeps it, and `nulled`, once every call is seen, takes out each parameter
given one that isn't so, again until none is). It's this amendment's own
rule for the crate's calls, followed through the parameters they give on;
no call is given `?? null`, which is a consumer's of another crate's. A
parameter of what isn't a candidate, a closure's `function::with_this`
binding, is as declared, as before: what calls it is JS.

Case ([0262](0262-when-rust-and-js-disagree.md)): results change, so Rust's
behavior: a `None` JS leaves out is `None`.

- Held by corpus `nullable_param_passed_on` (the three functions, called
  with `None`, `Some` and a callback, and a chain each of whose calls gives
  `null` or a value, still `=== null`, against native Rust),
  `test/lowering-nullable-reads.test.ts`'s "a nullable parameter given on
  what JS may leave out is tested against undefined too" (the public
  function called from Node with its arguments left out), and the
  mutations `given-on-taken-as-nulled` (`lower/analysis/nulled_params.ts`)
  and `given-on-param-unsaid` (`lower/mir/rvalues.ts`).

## Amendment: a function's every return, a bind, a variable's every write and an `as_deref()` (2026-11-03, rd632)

Five tests in React's port were `!= null` where upstream has `!== null`,
each of a parameter whose chain of calls is followed to a source that
`stored_given` couldn't say is `null` or a value. Each is followed now.
Each is also exactly as sound as its source: JS gives such a parameter
only `null` or a value.

**A function of the crate's whose every return is `null` or a value
gives so** (`NulledParams::returns_nulled`). React DOM's `getEventTarget`
returns `node.parent_node().map(|p| &**p)`, a nullable binding's result
mapped to itself, or `Some(target)`. ScrollEndEventPlugin's
`nativeEventTarget` is given its call. Lowering a function's body says
whether each write of its result is so (`mir/nulled.rs`'s
`note_nulled_returns`). It must be a whole assignment or a call's
result, never lent as a `&mut` nor a variant set, and each must be
`stored_given`'s `Nulled` of no parameter. A function that returns its
own nullable parameter isn't counted: a call of the crate's may give
that parameter `undefined`. A copy of the function, an `async` one, a
writer, a binding, a trait's method, and one already
`#[rust_js::nullable]` (`may_return_nulled`) are skipped. A call of such
a function is `Nulled` wherever `stored_given` asks. A body that asked
before the function was found is lowered again (`again`) before any
parameter is found. What it saw of calls is seen anew: `NulledCalls`
keeps each record by the item whose lowering saw it. What's found only
grows, so each lowering again only finds more.

**A `function::bind` of a function with each of its parameters bound is
a call of it, given what's bound**: React's
`scheduleTimeout(completeRoot.bind(null, root, .., suspendedCommitReason))`.
Before, the function was used as a value (`as_values`), so none of its
parameters was ever found. `scan` (`bound_all`) counts the bind as the
function's call site where the tuple has as many items as the function
has parameters. A bind of some parameters leaves the rest to whatever
calls it, and it's still a use as a value. Lowering records what each
item of the tuple made at the bind gives (`record_bound`).

**A variable written more than once is `null` or a value where every
write is**, each whole, nothing lending it as a `&mut`: `if profiling {
suspended_reason() } else { js::null() }`'s temporary. A variable written
once is as before. One write of a `None` it makes, `undefined`, and it
isn't so.

**A shared `as_ref()` or `as_deref()` of such is so**, the option itself
in JS (Std::Pointee, ADR 0023): commitPlacement's
`parentFragmentInstances.as_deref()` of `getParentFragmentInstances`'s
nullable result. An `as_mut()` isn't counted, as its items may be boxed.

Case ([0262](0262-when-rust-and-js-disagree.md)): results change, so
Rust's behavior: a test `=== null` only of what's never `undefined`.

Held by corpus `nulled_returns_given_on` (each source and each loose
kind, against native Rust), and by `test/lowering-nullable-reads.test.ts`'s
"a function whose every return is null or a value returns null or a
value" (`run` is lowered before `event_target`, so it's lowered again)
and "a bind of each parameter is a call of the function" (a bind of all,
of some, and of what JS gave). Mutations `returns-nulled-unfound`,
`returns-nulled-unasked`, `returns-own-param-nulled`,
`writes-nulled-any`, `writes-nulled-unfollowed`, `as-deref-unfollowed`,
`bound-tuple-unnulled` and `bound-call-unrecorded`
(`lower/mir/nulled.ts`), and `returns-askers-never-again`,
`returns-askers-calls-kept`, `bind-a-value-use` and
`bind-of-some-parameters-a-call` (`lower/analysis/nulled_params.ts`).
`stored_from` and what it asks moved from `mir/rvalues.rs` to
`mir/nulled.rs`, along with their mutations.

## Amendment: a `let` a call writes, read by a nullable parameter and alike, is `null` for `None` (2026-11-03, rd632, the user's decision (B))

The fifth site was really `undefined`. React's measureUpdateViewTransition
keeps `let previous_measurements = if gesture { clones.as_ref().map(..) }
else { let previous_measurements = js::cast(old_fiber.memoized_state.get());
..; previous_measurements };`. Its JS is `state.clones?.map(..)`, which is
`undefined` where `clones` is `null`. The `let` is given
`measureViewTransitionHostInstances` as `previous_measurements.as_deref()`
and read by `map_or(0, |p| p.len())`. The user chose (B) over accepting
`!= null`.

**A `let` a call writes is `null` for `None` where it's written when a
nullable parameter of the crate's own reads it, and every other read is
one too or reads `null` and `undefined` alike** (`mir_null_nones`'s `(B)`
branch, `nulled::alike_reads`). The reads it takes are a shared `as_ref()`
or `as_deref()` given on to such a parameter, `is_some()`, `is_none()`, a
`match`'s or `if let`'s test, a `Some`'s value read, and `map_or`. The
`let` is a variable the program wrote, not MIR's temporary. Each of its
arms' own `let`s or temporaries, written once and read once there, is
written so too. Its writes are then `previousMeasurements =
state.clones?.map(..) ?? null;` and `previousMeasurements =
oldFiber.memoizedState ?? null;`. Upstream writes `previousMeasurements =
null` in an `if` and `= oldFiber.memoizedState`, a field TypeScript types
`any`. Any other read keeps the `let` as it was, what the call gave,
`undefined` too: a binding's parameter that isn't nullable (JS's
`String(undefined)` is "undefined"), a return, a field, a capture, or a
`&mut`. A temporary given straight to a parameter, `f(g())`, is as it was,
so no `?? null` is written where no `let` is.

**A call's result written where a nullable place takes it is as it is
where `stored_given` says the call gives `null` or a value**
(`nulled::call_nulled`, which replaces rd628's `gets_constructed` at the
call's destination). A function found to return so is one: React DOM's
`return diffHydratedProperties(..)`, and `const nativeEventTarget =
getEventTarget(nativeEvent);` with no `?? null`.

**A closure that captures nothing and gives its one argument back as the
same JS value maps an option to itself**
(`std_types/option.rs`'s `maps_to_itself`, `closure_gives_its_argument`).
Each step must be a copy, a reborrow, or a call that's the value itself
(`Std::Same`, a JS object's `Deref` to its base, `Std::Pointee`), with no
branch and no other call or statement. This is React DOM's
`parent_node().map(|parent_node| &**parent_node)`. Its lowering already
printed the option alone, by the arrow's JS (`is_identity`). Now
`stored_given` knows it too, so `getEventTarget` is found to return `null`
or a value. One answer serves both: the map's lowering asks
`maps_to_itself` first.

**What a binding that's its argument in JS gives, of an `Option`, is what
it's given**: `js::cast(Some(js::unknown(options)))`, whose `Some` is
never `undefined`. React DOM's hydrateRoot's `match options {
Some(options) => js::cast(Some(js::unknown(options))), None => js::null() }`
is `options ?? null` alone. Without it, (B)'s `?? null` at the cast's write
made it `options != null ? (options ?? null) : null` (held by "a cast of
an Option is what it's given", mutation `cast-of-a-nulled-unfollowed`).

Case ([0262](0262-when-rust-and-js-disagree.md)): results change, so
Rust's behavior. A `None` given a parameter tested `!== null` is `null`;
where JS can tell `null` from `undefined`, it's as Rust's code made it.

Held by corpus `nullable_let_read_alike` (the widened `let` and each
excluded read, against native Rust; the binding `String` says
"undefined"), and by `test/lowering-nullable-reads.test.ts`'s "a let a
call writes, read by a nullable parameter and alike, is null for None"
and "a closure that gives its argument maps an option to itself". Mutations
`let-alike-of-a-temporary`, `let-read-otherwise-nulled` and
`let-arms-uncounted` and `let-arms-unnulled` (`lower/mir/patterns.ts`), `alike-as-deref-unsaid`,
`alike-is-some-unread` and `alike-map-or-unread` (`lower/mir/nulled.ts`),
and `closure-map-unrecognized` and `closure-map-effects-dropped`
(`lower/std_types/option.ts`). Each excluded read is a read `alike_reads`
doesn't count, so the corpus case holds it rather than a mutation of a
condition of its own.

## Amendment: a late `let` each arm writes counts each write once (2026-11-03, rd647 (C))

React's measureUpdateViewTransition, ported in upstream's shape, sets a late `let` in each arm:

```rust
let previous_measurements: Option<Vec<&'static Unknown>>;
if gesture {
    match clones.as_ref() {
        None => { previous_measurements = None; }
        Some(clones) => { previous_measurements = Some(..collect()); }
    }
} else {
    previous_measurements = unsafe { js::cast(old_fiber.memoized_state.get()) };
    ..
}
```

It's read by the nullable parameter and by `map_or`, as the amendment above
asks. Still, its `None` printed `undefined`, where upstream writes `null`.
MIR writes each arm into a temporary and copies it in. The rule counted
the `Some(..)` temporary twice, once as a write of a `Some` it moves in
and once as an arm. It also counted each arm once per copy MIR records,
two for these arms. So its writes never added up. Each arm is now counted
once (a set), and one whose `Some` is moved in already is not counted
again. It prints `previousMeasurements = null;`, upstream's, and the
`match` keeps its arms, `if (clones == null) { .. = null; } else { .. }`,
upstream's shape. The case is ADR 0262's "no observable difference":
every reader tests `null` and `undefined` alike.

Held by corpus `nullable_let_set_in_arms` against native Rust, and
mutations `let-arm-some-counted-twice` and `let-arms-counted-each-copy`
(`lower/mir/patterns.ts`); `let-arms-uncounted` is retargeted to the new
count. Now that the count adds up, rd623's `let` of `js::null()` would be
`null` by this rule as well. So `null-call-called` is held by a temporary
instead, `either(if c { js::null() } else { None })`'s `either(null)`.
Only the rule that `js::null()` isn't a call that may give `undefined`
makes it so, because a temporary isn't a `let`.
