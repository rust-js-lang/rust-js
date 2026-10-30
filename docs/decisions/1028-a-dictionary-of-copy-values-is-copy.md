# 1028. A dictionary or a union of `Copy` values is `Copy`, and a narrower union is one of a wider

Status: Accepted (rd208, a port gap; P2). Extends [0102](0102-js-and-webapi.md)'s
dictionaries, [0215](0215-webapi-unions.md)'s unions and
[1023](1023-a-dictionary-is-a-js-object-and-its-parents.md)'s dictionary that
extends a member.

Case ([0262](0262-when-rust-and-js-disagree.md)): none. A copy of an
options object is an object of the same fields, which the browser reads;
a union's value is its member itself, so a value of a narrower union is a
value of the wider.

## Context

React DOM's FragmentInstance keeps the options a listener was added with,
`{type, listener, optionsOrUseCapture}`, and gives them to each child's
`addEventListener` and, later, `removeEventListener`. webapi's
`AddEventListenerOptions` was neither `Copy` nor `Clone`, so the port
rebuilt it, a new object of the same four members, and its
`AddEventListenerOptionsOrBool` wasn't an `EventListenerOptions |
boolean`, what `removeEventListener` takes, which TypeScript's
`AddEventListenerOptions | boolean` is.

## Decision

- **A dictionary or a union a function takes, each of whose values is a
  number, a `bool`, a borrow, or an `Option`, dictionary or union of them,
  derives `Clone` and `Copy`**: 396 of webapi's. One that holds a
  callback, a `Box`, isn't.
- **A union whose each member is a member of another is one of it**, an
  `impl IntoEventListenerOptionsOrBool for AddEventListenerOptionsOrBool`,
  as ADR 1023's dictionary that extends a member is: `StrOrNumber` a
  `BoolOrNumberOrStr`, `BufferSource` a `BodyInit`.

```rust
let options = listener.options;
target.add_event_listener_named(&listener.kind, listener.listener, Some(&options));
..
target.remove_event_listener_named(&listener.kind, listener.listener, Some(&listener.options));
```

```js
const options = listener.options;
target.addEventListener(listener.kind, listener.listener, options);
..
target.removeEventListener(listener.kind, listener.listener, listener.options);
```

## Consequences

- Held by `test/webapi-bindings.test.ts`'s "listener options kept are
  given again, to add and to remove" (failing first: the options moved
  out of a borrow, and no `IntoEventListenerOptionsOrBool`). Mutations:
  none; the webapi crate's types are what rustc checks (ADR 0093).
