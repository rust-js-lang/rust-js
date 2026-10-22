# 0613. `&&` and `||` mixed are one value, as written

Status: Accepted. Extends tidy's logicals (`logical_returns`).

Case ([0262](0262-when-rust-and-js-disagree.md)): A: the same tests, in
the same order, short-circuited as they were.

## Context

MIR lowers `&&` and `||` to branches; tidy folds them back where they're
a chain of one kind, `a || b`. Mixed, `(x == y && (x != 0.0 || 1.0 / x ==
1.0 / y)) || (x.is_nan() && y.is_nan())`, React's `Object.is` polyfill,
was MIR's block of its tests:

```js
bb8: {
  if (x === y) {
    if (x === 0) {
      if (1 / x !== 1 / y) {
        break bb8;
      }
    }
    return true;
  }
}
return Number.isNaN(x) && Number.isNaN(y);
```

## Decision

**A labelled block of tests a logical the program wrote is, followed by
the `return` it falls to, is the one value returned**
(`branches::logical_blocks`): each `return` and `break` the value of what
follows it, each `if` folded as `conditional` folds one, and a value both
sides end with taken out, `c ? (p || d) : d` as `(c && p) || d`. Not where
the value after it would be written twice, a conditional is left, or the
program wrote a `return` there.

## What holds it

- `test/lowering.test.ts`, "a value of && and || mixed is returned as
  written", each of sixteen inputs of `(a || b) && (c || d)` too.
- Mutations: the block kept, unfactored.
