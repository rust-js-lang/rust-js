# 0613. `&&` and `||` mixed are one value, as written

Status: Accepted. Extends tidy's logicals (`logical_returns`). Amended:
`logical returns` runs again once MIR's temporaries are read in place (rd179).

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

## Amendment: once its reads are in place (rd179, 2026-10-30)

React DOM's isSuspenseInstanceFallback, `instance.data() == FALLBACK ||
(instance.data() == PENDING && instance.owner_document().ready_state() !=
LOADING)`, reads a getter in each test, which MIR holds in a temporary,
`const _3 = instance.data; if (_3 === FALLBACK) { return true; } const _11
= ..`: `logical returns`, in the `logicals` stage, saw statements between
the `if` and what follows, and joined nothing. Once `in place` reads the
temporaries where they're used, `branch values` made the `&&` but nothing
the `||`: `if (instance.data === FALLBACK) { return true; } return
instance.data === PENDING && ..;`.

**`logical returns` runs again in `in place`, after `inlined in values`**:
`return instance.data === FALLBACK || (instance.data === PENDING &&
instance.ownerDocument.readyState !== LOADING);`, as upstream writes it.
The pass is the same rule, one owner, run where its input is ready; no
corpus case or port file changes otherwise.

Held by `test/lowering-returned-chains.test.ts`, "a returned || of
getters' reads is returned as written", and the mutation
`in-place-logical-returns-off` (`mir/tidy/stages.ts`).
