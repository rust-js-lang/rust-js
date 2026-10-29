# 1021. A listener is a closure or one JS gave, each as it is

Status: Accepted (the user's decision, rd185). Amends
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
