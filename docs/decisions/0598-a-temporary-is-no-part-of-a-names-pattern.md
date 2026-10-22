# 0598. A temporary is no part of a named variable's pattern

Status: Accepted. Amends tidy's destructuring
([0293](0293-cells-read-at-once.md)).

Case ([0262](0262-when-rust-and-js-disagree.md)): A: one pattern or two
reads of the same properties, in the same order.

## Context

React's scheduler sets up its channel:

```js
const channel = new MessageChannel();
const port = channel.port2;
channel.port1.onmessage = performWorkUntilDeadline;
```

Its Rust, `let port = channel.port2(); channel.port1().set_onmessage(..)`,
became

```js
const { port2: port, port1: _12 } = channel;
_12.onmessage = ..
```

Two parts of a variable the program names, read in a row, are one
pattern, `let (a, b) = t;`'s. But the second was MIR's temporary for
`channel.port1()`, which the program never named, and would have been read
in place.

## Decision

**Of a variable the program names, a pattern takes the parts it names**:
a temporary among them ends the run, and is read where it's used,
`channel.port1.onmessage = ..`. A temporary's own parts, `const [a, b] =
f();`, are as before.

## What holds it

- `test/lowering.test.ts`, "a temporary is no part of a named variable's
  pattern".
- Mutation: a temporary's part kept in the pattern.
