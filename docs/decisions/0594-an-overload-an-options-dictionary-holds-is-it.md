# 0594. An overload whose arguments an options dictionary holds is that one

Status: Accepted. Amends [0443](0443-a-value-a-caller-may-leave-out-is-an-option.md),
whose overloads that differ in a parameter stayed functions of their own.

Case ([0262](0262-when-rust-and-js-disagree.md)): none; what a binding
takes, which JS's function takes either way.

## Context

React's scheduler posts to a `MessageChannel`'s port, `port.postMessage(null)`.
WebIDL declares two overloads, `postMessage(message, transfer)` and
`postMessage(message, optional StructuredSerializeOptions options = {})`;
webapi's generator kept the first and dropped the second as a clash of
names, so a Rust program had to give `transfer`, `&[]`, which JS doesn't
ask for. A `_with_` name for the second is forbidden (ADR 0443: one
function for one JS function). `Window`'s is the same:
`postMessage(message, targetOrigin, transfer?)` beside `postMessage(message,
options?)`, whose `WindowPostMessageOptions` holds `targetOrigin` and
`transfer`.

## Decision

**Of two overloads of one name, the one whose arguments past the other's
are each a member of the other's options dictionary is dropped, the
dictionary's kept** (`subsumedOverloads`): the newer form, which says
everything the older one does.

```rust
port.post_message(1, None);
let options = WindowPostMessageOptions { target_origin: Some("*"), ..Default::default() };
window.post_message(message, Some(options));
```

**A dictionary's member that's a list of objects, `transfer: sequence<object>`,
is a slice of them**, `&'a [&'a dyn Any]`, as a parameter's is; only one
typed `any`, which a struct field can't be generically, is left out.

Others it changes: `structuredClone(value, options?)` takes its options;
`CanvasRenderingContext2D.setTransform` its `DOMMatrix2DInit`;
`CookieStore.get` and `getAll` their `CookieStoreGetOptions`.

## Why

- **One function, as the platform's newer form**: what MDN shows first,
  and a superset of the older's arguments.

## What holds it

- `test/bindings.test.ts`, "webapi's any parameters take any value as JS
  has it": `window.postMessage(text, options)`, `port.postMessage(1)`.
- `test/webapi-coverage.test.ts` and the generated crate's test.
