# 0617. `Array.isArray` and `Object.getPrototypeOf` are the js crate's

Status: Accepted. Extends [0102](0102-js-and-webapi.md)'s js crate.

Case ([0262](0262-when-rust-and-js-disagree.md)): A: the bindings are JS's.

## Context

React's `shared/isArray.js` and `getPrototypeOf.js` hold JS's own
functions, `const isArrayImpl = Array.isArray;`, `const getPrototypeOf =
Object.getPrototypeOf;`. The js crate bound neither.

## Decision

**`js::array::is_array(value)` is `Array.isArray(value)`, and
`js::object::get_prototype_of(value)` is `Object.getPrototypeOf(value)`**,
`None` of an object that inherits from none. Held as a `fn`, each is the
function itself (ADR 0602).

`js::classify`'s `Kind::Array` stays what a value's kind is, to `match`;
`is_array` is JS's function, which a program may hold or pass, as React's
`isArray` holds it.

## What holds it

- `test/lowering.test.ts`, "Array.isArray and Object.getPrototypeOf are
  held as JS holds them".
