# 0293. A `Cell` a `let` takes apart and reads at once is its value

Status: Accepted. Amends [0288](0288-cell-fields.md): what a `let` binds of
a `Cell` field can be its value.

Case: A ([0262](0262-when-rust-and-js-disagree.md)).

## Context

A `Cell` in a field is the property (ADR 0288), and what's bound of it is
a handle on it: a `get()` later reads the property then, as Rust reads the
cell then. So a `let` that took one apart couldn't be JS's destructuring,
which reads every property at once. react.dev's ErrorMessage destructures
the error it shows, whose title Preview may change:

```ts
const {message, title} = error;
return <h2>{title || 'Error'}</h2> ...
```

Its Rust, `let ErrorType { message, title, .. } = error;` with
`title.get()`, read `error.title` and `error.message` where they're used.

## Decision

**A `&Cell` a `let` binds of a field, read only by `get()` before anything
else runs, is the value in JS's destructuring, and its `get()` reads it:
`const { message, title } = error;`.**

"Before anything else runs", in the order the code runs from the `let`: no
call, which could set the cell, and no block, whose drops could, ends
before the `get()`; the `get()` isn't in a loop, where a later turn would
read it again. Then the value the destructuring read is what `get()` gives.

A `&Cell` used otherwise, passed on, set, or read in a closure, is still a
handle.

## Why

- **It's the JavaScript a person writes**: the original's destructuring.
- **It's the same program**: nothing could set the cell between the `let`
  and the read.
