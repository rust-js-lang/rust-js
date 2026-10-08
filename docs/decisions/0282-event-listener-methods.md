# 0282. One method API for event listeners

Status: Accepted. Amends [0024](0024-web-crate.md),
[0223](0223-webapi-event-and-tag-maps.md) and [0260](0260-webapi-callbacks.md).

Case: N ([0262](0262-when-rust-and-js-disagree.md)).

## Context

A button's listener was typed correctly, but required a module call and a
box around the closure. Explicit removal also consumed a box: a caller
could not simply pass the same owned callback to add and remove.

ADR 0024 originally ruled out methods because dependency bodies were not
available. Attribute-backed methods now work from crate metadata, as the
React bindings already demonstrate. A prototype with Rust 1.99.0 verified
this without a compiler change; it does not imply a new rustc feature.

The API should have one spelling for each job. Keeping both module and
method calls, or separate registration methods for reusable callbacks,
would make users choose between equivalent operations.

## Decision

Keep the event markers and target-dependent `Listen<E>` mapping. Use one
`EventTargetExt` trait, implemented for every `IsA<EventTarget>` receiver:

```rust
use webapi::EventTargetExt;
use webapi::events::Click;

button.add_event_listener(Click, |e| {
    let x = e.client_x();
});
```

The trait's bound is `Self: Listen<E>`, so the receiver stays a button and
`e` is a `PointerEvent`. Putting the method only on the base `EventTarget`
would lose that information through `Deref`. A keyboard listener still gets
a `KeyboardEvent`; a name absent from the target's map is still an error.

- Add and remove take `impl FnMut(&EventType) + 'static`. An inline closure
  can own mutable captured state. The explicit `'static` keeps the old box's
  lifetime rule: a retained callback cannot borrow temporary stack data.
- The `_with_options` methods accept a boolean or the existing options
  dictionary directly, using the existing union conversion traits. They
  preserve `capture`, `once`, `passive` and `signal` as given.
- Unknown event names use the explicit `_named` variants, whose callback
  receives the base `Event`. Both add and remove have these variants.
- `EventTarget::dispatch_event` and the instance members of `Event` and its
  descendants are inherent methods. `e.client_x()`, `e.key()` and
  `e.prevent_default()` are property reads and calls, with inherited members
  supplied by `Deref`. Constructors, constants and unchecked casts stay in
  their modules. Other DOM operations are outside this change.
- The old module-level listener, dispatch and event instance functions are
  removed. There are no deprecated aliases or parallel `add_listener` API.
- All methods are declarations through `rust_js::link_name`, with no
  dependency function bodies or runtime wrappers in the output.

For a callback reused by add and remove, create a shared function once:

```rust
use webapi::{EventTargetExt, PointerEvent, listener};
use webapi::events::Click;

let callback = listener::<PointerEvent>(|e| {
    let x = e.client_x();
});
button.add_event_listener(Click, callback);
button.remove_event_listener(Click, callback);
```

`listener` takes `impl Fn(&E) + 'static` and returns `&'static dyn Fn(&E)`.
This is an ordinary Rust callable type: its shared reference also satisfies
`FnMut`, so the same registration methods accept it without a new callback
trait or overload. A shared callback needs `Fn`; use `Cell` or `RefCell` for
shared mutable captured state. Do not pretend a directly mutable `FnMut`
closure is a shared `Fn`.

The factory's identity binding emits the function itself. Sharing its
reference never wraps or clones it. JavaScript's garbage collector owns
its runtime lifetime, as with other webapi factories returning `'static`
references. It does not arrange removal on scope exit.

```js
const callback = (e) => {
  const x = e.clientX;
};
button.addEventListener("click", callback);
button.removeEventListener("click", callback);
```

Capture flags must match when removing. Constructing a different closure
with the same body does not remove the first one. `AbortSignal` remains
available when cleanup should remove several registrations together.

## Why

The binding generator owns this change. rustc still checks types and
lifetimes, and the compiler's existing binding lowering produces ordinary
JavaScript. A single extension trait avoids repeating listener methods on
hundreds of targets, while preserving their individual event maps.

Tests check emitted calls, rejection of removed aliases, wrong event/target
types and borrowed captures. Chromium dispatches events on buttons,
including inline and shared mutable captures, once, abort, custom events,
duplicate registration and capture-sensitive removal. The snapshot test
also covers the new fixture in the normal compiler verification workflow.

## Alternatives

- Keep module calls or add separate handle methods: two spellings for one
  browser operation, with more API to learn and maintain.
- Generate inherent listener methods on every target: avoids a trait import
  but duplicates the same declarations hundreds of times.
- Make only `EventTarget` have typed methods: loses the concrete receiver's
  map, or asks the caller to supply the event type manually.
- Convert arbitrary strings silently: gives up the existing typo check.
- A subscription with `Drop` cleanup: adds a lifecycle policy to the raw
  browser binding. Explicit removal and abort already cover the need.

## Consequences

- This is a source-breaking migration: import `EventTargetExt`, replace
  `event_target::add_event_listener(button, Click, Box::new(..))` with
  `button.add_event_listener(Click, ..)`, and use methods on events. Existing
  examples and the playground migrate with it, retaining their JavaScript.
- Options dictionaries are passed directly; remove the old `.into()` where
  a typed listener's options previously needed its explicit union enum.
- A separately created shared callback sometimes needs its event type
  stated, hence `listener::<PointerEvent>`. It must own its captures.
- No callback type, custom callable-trait implementation, runtime helper,
  or new compiler lowering is needed. Generated methods use the same
  binding metadata on the native and browser-hosted compiler paths.
- This changes no event delivery or `FnMut` reentrancy contract. The browser
  still owns dispatch; the shared callback factory preserves identity.

## Since

- **Removal takes a shared callback, `listener(..)`'s**, `&'static dyn
  Fn(&Event)`, not any closure: removal finds the function added, and a
  closure made for the call is a new one, which removes nothing, so
  `button.remove_event_listener(Click, |_| {})` is a type error, not a call
  that does nothing. The `_named` and `_with_options` forms too.
- **Its browser test runs with the other browser tests**, after the rest of
  the suite (`scripts/test.ts`): it opens a page per case in Chromium.
