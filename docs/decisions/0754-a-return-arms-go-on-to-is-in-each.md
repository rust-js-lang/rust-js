# 0754. A `return` some arms go on to is in each, not a block's end

Status: Accepted. Extends [0745](0745-a-continue-in-a-chain-is-where-it-is-written.md):
what a block MIR made ends with, put where each path that goes on to it
is, so the block's label goes.

Case ([0262](0262-when-rust-and-js-disagree.md)): it can't be observed:
which statements reach the same `return` is how the JS reads, not what it
does.

## Context

React's `getArrayKind` (shared/ReactPerformanceTrackProperties.js) is a
loop of an `if`/`else` chain whose arms return:

```js
if (typeof value === 'object' && value !== null) {
  if (isArray(value) && value.length === 2 && typeof value[0] === 'string') {
    // ..
    kind = ENTRIES_ARRAY;
  } else {
    return COMPLEX_ARRAY;
  }
} else if (typeof value === 'function') {
  return COMPLEX_ARRAY;
} else if (typeof value === 'string' && value.length > 50) {
  // ..
```

Its port is a `match` of `js::classify(value)`. MIR tests the kind, then
goes on from some arms to the one of `return ArrayKind::Complex` (`Kind::
Object(_) | Kind::Array(_)`, and an array that isn't a pair), and past it
from others (a string no longer than 50, a primitive) to the arm of `_ if
kind != ..`. The JS kept the block MIR made for it:

```js
bb12: {
  if (typeof value === "string") {
    if (value.length > 50) {
      return "Complex";
    }
    break bb12;
  }
  // ..
  } else if (typeof value !== "object" || value === null) {
    break bb12;
  }
  return "Complex";
}
```

`branch_ends` makes an `else` of what follows an `if` whose `then` leaves
for the block's end, but not where the block ends with a `return` or a
`throw`: that's a `let … else`'s guard, `if (c) { s; break l; } throw e;`,
which reads as one.

## Decision

**A block MIR made that ends `if (c) { a } else { b } return v;`, of one
value, a constant or a name, is the `return` at the end of each path of
the `if` that goes on to it, each `break` of the block a path ends with
gone**, as the path now ends where the block does; an `if` of no `else` a
path ends with, whose `then` leaves, is given the `return` as its `else`
(`if (!c) { return v; }` where nothing's left of its `then`). Where a
`break` of the block would be left, deeper than a path's end, nothing
changes: the block is still needed. The `if` has an `else`: a `let …
else`'s guard has none, and stays one.

`getArrayKind` is a chain with no block MIR made:

```js
if (typeof value !== "string") {
  if (typeof value === "function") {
    return "Complex";
  }
  if (Array.isArray(value)) {
    if (value.length !== 2 || typeof value[0] !== "string") {
      return "Complex";
    }
    // ..
    continue;
  }
  if (typeof value === "object" && value !== null) {
    return "Complex";
  }
} else if (value.length > 50) {
  return "Complex";
}
```

## Alternatives

- **Each `break` of the block the code after it**: that's every statement
  after the block, in each place; the `return` is one value.
- **A `return` of any value**: a call's or a template's would be written
  out in each path; it stays at the block's end, which names it once.

## Consequences

- The `return` is written as many times as paths go on to it, as upstream
  writes `return COMPLEX_ARRAY` in each arm.
- The arms' order is still MIR's tests', the kind's variants', where
  upstream tests an object first.
- Held by `test/lowering-labels.test.ts`'s "a match's return some arms go
  on to is in each" and "a match's return of a call some arms go on to is
  written once", and the mutations `return-into-branches-never`,
  `return-into-branches-break-kept`, `return-into-branches-test-unturned`
  and `return-into-branches-of-any-value`.
