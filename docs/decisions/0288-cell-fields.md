# 0288. A `Cell` in a field is the property it holds

Status: Accepted. Amends [0023](0023-strings-references-shared-state.md):
a `Cell` of its own is still `{ value }`. Builds on
[0099](0099-mut-references.md)'s handles.

Case: A ([0262](0262-when-rust-and-js-disagree.md)).

## Context

A `Cell` is `{ value }`, so everyone sharing it sees a change (ADR 0023).
In a struct's field that was a box in the object, `{ count: { value: 1 } }`,
read `c.count.value`, which no person writes and no JS caller expects. A
JS object a binding describes can't be read that way at all: react.dev's
Preview retitles Sandpack's error in place, `rawError.title = 'Runtime
Error'`, and builds one of its own, `{ title, message }`, which its
`ErrorMessage` takes too.

ReScript's records are JS objects, and a field it marks `mutable` is the
property itself, `r.title = ..`. TypeScript's properties are writable.

## Decision

**A `Cell` in a struct's or an enum variant's field is the value it holds:
the property, read and set in place. A `&Cell` of one that leaves is a
handle on it (ADR 0099), so what it's lent sets the field.**

```rust
#[derive(Clone, Default, PartialEq)]
pub struct Counter { pub label: &'static str, pub count: Cell<u32> }

Counter { label: "a", count: Cell::new(n) }   // { label: "a", count: n }
c.count.set(c.count.get() + 1);               // c.count = (c.count + 1) >>> 0;
add(&c.count, 5);                             // add({ get value() { return c.count; }, set value(v) { c.count = v; } }, 5)
```

- A field read is a handle on it, and a handle's `value` is the place:
  every use of a cell, `.value`, is the property where it's read.
- What's made into a field is what the cell holds: `{ value: x }.value`
  is `x`.
- A derived `Clone` copies it with the object, `{ ...c }`; a derived
  `Default` is the value's default; a derived `==` compares the values.
- A constant's is its value: `const COUNTER = { hits: 0 }`.
- Taken apart, a struct with one isn't JS's destructuring, which would
  give the number: its fields are read where they are.
- Declared, it's what it holds, `count: number`; a `&Cell` lent is `{
  value: number }`.
- A tuple struct's, a tuple's and an array's are still `{ value }`: they
  are JS arrays, which ReScript doesn't make mutable either.

## Why

- **It's the JavaScript a person writes**, and the JS objects bindings
  describe already are.
- **It's the same program**: a field is one place, as the cell was, and
  what's lent it reaches it through a handle.

## Consequences

- Sandpack's `SandpackError` can be a struct with a `Cell` title, made as
  a literal, retitled in place and taken by `ErrorMessage`.
