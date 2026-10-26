# 0948. A value read and then stepped by one is `x++` where it's read

Status: Accepted. Extends [0631](0631-a-step-of-one-is-plus-plus.md), a
statement's step of one, to the step after a read of the value before it,
which 0631's alternatives left out because no compound's value is read.

Case ([0262](0262-when-rust-and-js-disagree.md)): N. `x++` of a number
reads `x`, writes `x + 1` and gives what it read: what the two statements
did.

## Context

React's `getViewTransitionName` takes an id from a counter:

```js
const globalClientId = globalClientIdCounter++;
```

Its port, a `Cell<f64>`'s `get` then `set`, wrote two statements:

```js
const globalClientId = globalClientIdCounter;
globalClientIdCounter = globalClientId + 1;
```

## Decision

**`const a = x; x = a + 1;`, of a path `x` (a variable, a field of one)
and the number literal `1`, is `const a = x++;`**, and `- 1` is `x--`.
Preparation makes it (`prepare::stepped`, after `compound`), a
`js::ExprKind::Stepped` the printer writes as JS's postfix update; nothing
before preparation makes one. Any other step, `a + 2`, stays as it was.

## Why

- **It's the JS a person writes** of a counter's next value.
- **The same reads and writes**: `x++` reads `x` once and writes once,
  after the read, as the two statements did; the literal `1` is only a
  number's (ADR 0631), whose `x++` is `x + 1`.

## What holds it

- `test/lowering-module-variables.test.ts`, "a counter read and then
  stepped is x++".
- Mutations: `post-step-unmade`, `post-step-of-any-number`,
  `post-step-down-is-up`.
