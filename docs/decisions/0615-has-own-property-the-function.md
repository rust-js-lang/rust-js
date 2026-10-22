# 0615. `Object.prototype.hasOwnProperty` is a function to call with its object

Status: Accepted. Extends [0102](0102-js-and-webapi.md)'s js crate.

Case ([0262](0262-when-rust-and-js-disagree.md)): A: the binding is JS's.

## Context

React's `shared/hasOwnProperty.js` keeps the function, and its callers
call it with the object they ask of:

```js
const hasOwnProperty = Object.prototype.hasOwnProperty;
..
hasOwnProperty.call(objB, currentKey)
```

which an object's own `hasOwnProperty`, or an object with no prototype,
can't change. The js crate bound neither the function nor its `call`.

## Decision

**`js::object::has_own_property` is `Object.prototype.hasOwnProperty`, a
static of the function, a `js::object::HasOwnProperty`, whose `call(object,
key)` is `.call(object, key)`**, as TypeScript types the function and
`Function.prototype.call`. Held where a module loads, a thread-local, it's
`const hasOwnProperty = Object.prototype.hasOwnProperty;`.

## What holds it

- `test/lowering.test.ts`, "Object.prototype.hasOwnProperty is called with
  its object": an object with its own `hasOwnProperty`, and one of no
  prototype.
