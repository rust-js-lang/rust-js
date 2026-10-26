# 0615. `Object.prototype.hasOwnProperty` is a function to call with its object

Status: Accepted. Extends [0102](0102-js-and-webapi.md)'s js crate.
Amended (2026-10-26): a value's own `hasOwnProperty`, `value.has_own_property(key)`,
is the other form, of the other meaning.

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

## Amendment: a value's own `hasOwnProperty` is the other form (2026-10-26)

React's `ReactFiberHydrationDiffs.js` asks each object it describes, in
seven places, of its own method:

```js
for (const propName in props) {
  if (!props.hasOwnProperty(propName)) {
    continue;
  }
```

The port could only write `object::has_own_property.call(props, key)`, as
this decision made it the one form. The two differ where a program sees
it: an object of no prototype, `Object.create(null)`, has no
`hasOwnProperty`, so upstream's call throws a `TypeError`, which
`describeDiff`'s `catch` makes `''`; `Object.prototype`'s answers `false`
of it, and the diff goes on. An object's own `hasOwnProperty`, which a
props object may have, answers in upstream's, and not in `.call`'s.

**`value.has_own_property(key)`, a method of `js::Unknown`, is
`value.hasOwnProperty(key)`**, the value's own method called, and so a
`JsObject`'s by its `Deref`. Each form is of its own meaning, and neither
is the other's synonym:

| Rust | JS | Of an `Object.create(null)` | Of an own `hasOwnProperty` |
|---|---|---|---|
| `value.has_own_property(key)` | `value.hasOwnProperty(key)` | a `TypeError`, a panic | its answer |
| `object::has_own_property.call(value, key)` | `Object.prototype.hasOwnProperty.call(value, key)` | `false` | `Object.prototype`'s |

A port writes what upstream writes: React's `shared/hasOwnProperty.js`'s
callers `.call`, its hydration diffs the method.

- **A method of `Unknown`**, as TypeScript has it of an object's apparent
  type, `Object`'s: what's in `for (const key in value)` is an `Unknown`,
  a `JsObject` one too. The js crate's free functions, `js::get`, are of
  what JS's syntax does, `value[key]`; this is a property's call.
- **A throw is a panic**, as every binding's is (ADR 0660), which
  `catch_unwind` catches, as upstream's `catch` does.
- Case N ([0262](0262-when-rust-and-js-disagree.md)): each is JS's own
  call, of what it's called on.

Held by `test/js-object.test.ts`, "a value's own hasOwnProperty is called
on it": an object of no prototype makes `describe_diff` `""`, an
inherited property isn't its own, and an own `hasOwnProperty` answers,
run; and by "Object.prototype.hasOwnProperty is called with its object",
whose same objects answer otherwise.
