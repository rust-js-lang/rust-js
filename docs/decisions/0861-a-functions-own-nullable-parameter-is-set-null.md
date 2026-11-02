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
