# 1023. A dictionary is a JS object, and a member of a union its parent is

Status: Accepted (rd37b). Extends [0229](0229-union-parameters.md) (a
union parameter takes each member as it is) to the dictionaries that
extend a member, and [0102](0102-js-and-webapi.md)'s dictionaries to
`js::Defined`.

Case ([0262](0262-when-rust-and-js-disagree.md)): none. JS is given the
object the program made, as it is.

## Context

React DOM's `checkPassiveEvents` tests whether the browser reads
`passive`: an options object whose `passive` is a getter, given as the
listener and the options to both calls.

```js
const options = {};
Object.defineProperty(options, 'passive', {
  get: function () {
    passiveBrowserEventsSupported = true;
  },
});
window.addEventListener('test', options, options);
window.removeEventListener('test', options, options);
```

Two of it couldn't be written. `remove_event_listener`'s options are
`Option<&dyn IntoEventListenerOptionsOrBool>`, which took an
`EventListenerOptions` alone, where TypeScript takes an
`AddEventListenerOptions` too, as it extends `EventListenerOptions`. And
a webapi dictionary wasn't `js::Defined`, so `js::unknown(&options)`, the
value to cast to the `EventListenerObject` the listener is (ADR 1021),
was refused, though a dictionary is a JS object (ADR 0020), never
nullish.

## Decision

- **A dictionary that extends a union's member is one**, its trait's
  member, as an interface that extends a member's class is: `impl
  IntoEventListenerOptionsOrBool for AddEventListenerOptions<'_> {}`, and
  `KeyframeAnimationOptions` a `number | KeyframeEffectOptions`. The value
  is the object itself; the enum, whose variant holds the member's own
  fields, has no `From` of it.
- **Every dictionary a function takes is `js::Defined`**: a JS object,
  so a value of any shape, `js::unknown(&options)`.

```rust
let options = AddEventListenerOptions::default();
object::define_property(&options, "passive", &PropertyDescriptor { get: Some(&|| { .. }), ..Default::default() });
// Safety: nothing dispatches "test", so JS never calls its `handleEvent`.
let listener: &EventListenerObject = unsafe { js::cast(Some(js::unknown(&options))) };
window.add_event_listener_named("test", listener, Some(&options));
window.remove_event_listener_named("test", listener, Some(&options));
```

```js
const options = {};
Object.defineProperty(options, "passive", { get() { .. } });
// Safety: nothing dispatches "test", so JS never calls its `handleEvent`.
const listener = options;
window.addEventListener("test", listener, options);
window.removeEventListener("test", listener, options);
```

## Why

Both are what TypeScript says of the same objects; the generator, which
owns the bindings, says it once of every dictionary. Chromium runs the
form above: the getter runs as `addEventListener` reads `passive`.

No mutation goes with it: the change is the webapi generator's, which the
mutation runner, a compiler's, doesn't mutate (ADR 0093); the test fails
to compile without each.

## Alternatives

- A `From<AddEventListenerOptions>` of the enum: it would copy the fields
  the member has, a new object, where JS is given the one with the getter.
- An `EventListenerObject` made of a dictionary by a function of its own:
  a second way to make one, where `js::cast` already says what the
  program promises.
