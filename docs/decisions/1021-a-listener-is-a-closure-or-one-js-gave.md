# 1021. A listener is a closure or one JS gave, each as it is

Status: Accepted (the user's decision, rd185); amended (rd192): an event handler property's setter takes a union too. Amends
[0282](0282-event-listener-methods.md) (one method API for event
listeners): a listener parameter is TypeScript's
`EventListenerOrEventListenerObject`, a union, so a closure's parameter
says its event; and applies [0229](0229-union-parameters.md) (a union
parameter takes each member as it is) to it.

Case ([0262](0262-when-rust-and-js-disagree.md)): none. JS is given the
listener the program gives, as it is: a closure's function, or the
function or object JS gave, whose identity removal finds.

## Context

`add_event_listener` and `remove_event_listener` took a Rust closure
alone, `impl FnMut(&E) + 'static` and `&'static dyn Fn(&E)` (ADR 0282).
React DOM's `FragmentInstance` is given a listener by the program that
uses it, `fragment.addEventListener(type, listener, options)`: a function
or an object of a `handleEvent` method, which it adds to each of its
nodes, those added later too, and removes from them by identity. Its
fragment-scoped `once` wraps it in a function that calls it as JS would,
`listener.call(this, event)` of a function, `listener.handleEvent(event)`
of an object. None of that could be written: a listener JS gave isn't a
Rust closure, and a closure made of it, `move |e| f(e)`, is a new
function, which removal never finds, and loses the `this` JS calls it
with.

## Decision

One `add_event_listener` and one `remove_event_listener` of each target,
and their `_named` forms, whose listener is a union, as TypeScript types
it:

- `add_event_listener(event, listener: impl IntoEventListener<E>, options)`
  takes a closure of the event, `listener(..)`'s shared one, a
  `Box<dyn FnMut(&Event)>`, a function `function::with_this` made, or a
  listener JS gave, an `EventListenerOrEventListenerObject` or an
  `&EventListenerObject`, each as it is.
- `remove_event_listener(event, listener: impl IntoAddedEventListener<E>,
  options)` takes one added before: `listener(..)`'s, or one JS gave. A
  closure made for the call is still refused, as it removes nothing.
- `EventListenerOrEventListenerObject` is an untagged enum (ADR 0215),
  `EventListener(&'static dyn Fn(&'static Event))` or
  `EventListenerObject(&'static EventListenerObject)`, told apart by
  `typeof listener === "function"`; its `==` is `===`, so a program finds
  the listener it keeps as upstream's `l.listener === listener` does.
  `EventListenerObject::handle_event(event)` is `object.handleEvent(event)`.
- A union gives rustc no closure signature (it deduces one only from an
  `Fn`-family bound), so **a closure's parameter says its event**:
  `button.add_event_listener(Click, |e: &PointerEvent| ..)`. An
  unannotated one is rustc's "type annotations needed"; one of another
  event, "type mismatch in closure arguments", naming the event expected.
- An event is a JS object, which a listener may keep, so a function a
  listener is takes a `&'static` event: a closure of `&E` is one, and so is
  `function::with_this(move |this, (event,): (&Event,)| ..)`, whose
  parameters are one type, not one of each lifetime.

```rust
pub fn wrapped(target: &EventTarget, kind: &str, given: EventListenerOrEventListenerObject) {
    let attached = function::with_this(move |this, (event,): (&Event,)| {
        match given {
            EventListenerOrEventListenerObject::EventListener(f) => {
                function::call(f, this.map(|t| t as &dyn js::IntoUnknown), (event,));
            }
            EventListenerOrEventListenerObject::EventListenerObject(o) => o.handle_event(event),
        }
    });
    target.add_event_listener_named(kind, attached, None);
}
```

```js
export function wrapped(target, kind, given) {
  function attached(event) {
    if (typeof given === "function") {
      given.call(this, event);
    } else {
      given.handleEvent(event);
    }
  }
  target.addEventListener(kind, attached);
}
```

React's `FragmentInstance` binding takes the same two traits.

## Why

One parameter per listener, the union TypeScript has, is one way to add a
listener whatever it is (AGENTS.md, "One way, one authority"); the
compiler needed no change, as a union's member is the value itself (ADR
0229). Every closure in rust-js, its examples, tests, guide and the react
crate now annotates its parameter; the JS is unchanged.

Chromium checks a function JS gave and a `handleEvent` object added,
removed by identity, and called through a wrapper, the function with the
target as its `this` and the object's method with the object as its own.
The diagnostics test checks the unannotated closure's and the wrong
event's errors, and removal of a closure made for the call.

No mutation goes with it: the change is the webapi generator's and the
react crate's bindings, which the mutation runner, a compiler's, doesn't
mutate (ADR 0093); the tests above fail without each impl.

## Alternatives

- A second pair for a listener JS gave, `add_event_listener_given`, as
  web-sys's `add_event_listener_with_event_listener` or TypeScript's
  overloads: keeps closure inference, but two methods do one job. The
  user chose the union.
- Nightly `fn_traits`, so a JS listener is itself an `FnMut`: webapi
  would need `RUSTC_BOOTSTRAP` natively, and an object isn't a function.
- A union trait with an `Fn` supertrait, which rustc deduces a closure's
  signature from: the enum and the object would have to be `Fn`, which
  only nightly allows.

## Amendment: an event handler property's setter takes a union too (rd192)

The user's decision (2026-10-29). A handler's setter, `set_onclick`,
takes `Option<&dyn IntoEventHandler<E>>`: a Rust function or a `Copy`
closure of the event, or any JS function of it or of any value, each as
it is, so a promise's `resolve` is one, `link.onload = resolve`. Borrowed,
not given, as an optional union is (ADR 0443), so a closure is `Copy`,
owning nothing it would drop while JS keeps it. See ADR 0653's amendment,
which says each handler's trait and why an element's `onerror` is an
`EventHandler`.

## Amendment: `listener(..)` takes a function of `&'static` events (rd209)

React DOM's FragmentInstance wraps a `once` listener in `function
(event) { .. listener.call(this, event) }`, keeps it, adds it to each
child and removes it by identity. Its port makes it with
`function::with_this(..)`, a `Box<dyn Fn(&'static Event)>`, which was no
`EventListener(&'static dyn Fn(&'static Event))`: `listener(..)` took only
a closure of every lifetime's event, `impl Fn(&E)`.

**`listener<E: 'static>(impl Fn(&'static E) + 'static) -> &'static dyn
Fn(&'static E)`**, the event `&'static` as every listener's is (an event
is a JS object a listener may keep): one way to make a JS-owned listener,
of a closure or of a `with_this` function. A closure's event is inferred
`&'static` as before. Removal takes the shared listener so typed; one of
every lifetime, a handler getter's, is one so typed, as it coerces, `f as
&dyn Fn(&'static Event)` (two impls, of both types, overlap: rustc's
coherence warns of them).

```rust
let attached = listener(function::with_this(move |this, (event,): (&'static Event,)| {
    if let EventListenerOrEventListenerObject::EventListener(f) = given {
        function::call(f, this.map(|t| t as &dyn js::IntoUnknown), (event,));
    }
}));
let attached = EventListenerOrEventListenerObject::EventListener(attached);
```

```js
function attached(event) {
  if (typeof given === "function") {
    given.call(this, event);
  }
}
```

Held by `test/webapi-events.test.ts`'s "a function of its this is kept as
a listener, the function itself" (`kept_wrapped`, run: added to two
targets, the one function, called with its `this`). No mutation: the
generator's.

## Amendment: a shared listener is `&'static dyn Fn(&E)` again; a box kept is `Box::leak(..)`'s (rd209's regression)

The amendment above typed `listener(..)` `&'static dyn Fn(&'static E)`,
and removal took only that. React DOM's port types its listeners as a
Rust engineer types a function of the event, `&'static dyn Fn(&Event)`,
of every lifetime: `remove_event_listener(target, type, listener:
&'static dyn Fn(&Event), capture)`, a listener assigned over the one
given, `listener = webapi::listener(move |event: &Event| ..)`, and one
returned, `-> &'static dyn Fn(&AnyNativeEvent)`. None compiled: such a
`dyn` isn't one of `&'static` events (rustc's "not general enough"), and
one of `&'static` events doesn't coerce to it.

No one type serves both. A function of `&'static` events, a
`with_this(..)` box's, is no function of every lifetime's (a closure that
keeps the event couldn't soundly be called with a borrowed one), and
`listener(..)` giving what it was given, `&'static F`, makes a listener
that removes itself, `Rc<Cell<Option<..>>>` captured by its own closure
(React DOM's setSrcObject), a type of infinite size. Two impls of
removal, one for each `dyn`, differ only in whether a lifetime is
higher-ranked, which rustc's coherence warns of as a future error
(`coherence_leak_check`, rust-lang/rust#56105).

**`listener<E>(impl Fn(&E) + 'static) -> &'static dyn Fn(&E)`**, as before
rd209: a closure of `&E` is the listener, of every lifetime. **Removal
takes a reference to any function of the event JS keeps**, `impl<E:
'static, F: ?Sized + Fn(&'static E) + 'static> IntoAddedEventListener<E>
for &'static F`, one impl: `listener(..)`'s, `&'static dyn Fn(&'static
Event)` an `EventListener` gave, a named `fn`'s. A closure made for the
call, given by value, still isn't one. A function already made, a
`with_this(..)` box, is kept for good as any box is, `Box::leak(..)`,
which is the function itself (`Box::leak` is the value, ADR 0238), and
an `EventListener` as it is:

```rust
let attached = Box::leak(function::with_this(move |this, (event,): (&'static Event,)| { .. }));
let attached = EventListenerOrEventListenerObject::EventListener(attached);
```

prints `function attached(event) { .. }` as before. `listener(..)`
makes a function of a closure; `Box::leak` keeps one that's made: each
is the one way for what it's given.

Held by `test/webapi-events.test.ts`'s "event methods and reusable
listeners emit direct JavaScript" (`unsubscribed`: a `&'static dyn
Fn(&Event)` given, removed, assigned over by `listener(..)` and returned)
and "a function of its this is kept as a listener" (`kept_wrapped`, now
`Box::leak`), and by building react-in-rust's react-dom-bindings, whose
four sites compile as written. The refusal of a closure that names a
different event is rustc's closure-signature error now, where the
`EventListenerOrEventListenerObject` note was. No mutation: the
generator's.

## Amendment: a with_this function is a listener as it is, no `Box::leak` (rd209)

The amendment above kept a `with_this(..)` box with `Box::leak(..)`, a
leak a port mustn't write. `with_this` now gives the JS function itself,
`&'l dyn Fn(..)` (ADR 0972, amended), so it's an `EventListener` as it
is:

```rust
let attached = function::with_this(move |this, (event,): (&'static Event,)| { .. });
let attached = EventListenerOrEventListenerObject::EventListener(attached);
```

`listener(..)` makes a JS-owned function of a closure; `with_this(..)` one
of a closure given its `this`: each the one way for what it's given. Held
by `test/webapi-events.test.ts`'s `kept_wrapped`, the same JS.
