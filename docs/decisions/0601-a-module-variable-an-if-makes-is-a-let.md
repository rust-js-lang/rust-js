# 0601. A module's variable an `if` makes is a `let` each branch assigns

Status: Accepted. Extends [0270](0270-module-variables.md)'s
thread-locals. Amended by [0657](0657-an-if-of-one-value-each-is-a-conditional.md):
an `if` of one value each, a function too, is a `const` of a conditional.

Case ([0262](0262-when-rust-and-js-disagree.md)): A: the same value, made
once when the module loads.

## Context

React's scheduler picks how it schedules work when it loads:

```js
let schedulePerformWorkUntilDeadline;
if (typeof localSetImmediate === 'function') {
  schedulePerformWorkUntilDeadline = () => { .. };
} else if (typeof MessageChannel !== 'undefined') {
  const channel = new MessageChannel();
  ..
} else {
  ..
}
```

Its Rust is a thread-local an `if` makes. A value made of statements is a
closure called where it's made, so it was

```js
const schedulePerformWorkUntilDeadline = (() => {
  if (typeof setImmediate !== "undefined") {
    return () => { .. };
  }
  ..
})();
```

## Decision

**A module's `const` a closure called where it's made makes, not
exported, is `let name;` and the closure's body**, each `return v` at its
ends `name = v`, an early `return`'s `if` the rest's `else`. What follows
stays as JS writes it: `let x;` and what sets it are one block, no blank
line between.

It stays the call where the body declares a name at its top, which would
be the module's, and where a `return` isn't at its ends, a loop's.

The variable is a `let`, which nothing else assigns: Rust's thread-local
is set once, and JS has no `const` set in branches.

## What holds it

- `test/lowering.test.ts`, "a thread-local an if makes is a let each
  branch assigns", with one that declares at its top and one of a loop,
  which stay calls.
- Mutations: the pass off, a top declaration taken, the blank line kept.

## Amendment: `undefined` is no assignment (ADR 0700)

A `return undefined` at an end, an `Option`'s `None`, assigns nothing:
`let name;` is `undefined` already, so `else { name = undefined; }` is
gone, and a `then` left with nothing is the `else`'s under its test
negated ([0700](0700-a-module-variable-is-assigned-only-what-isnt-undefined.md)).
