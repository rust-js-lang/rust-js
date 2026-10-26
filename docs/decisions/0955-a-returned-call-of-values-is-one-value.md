# 0955. A `return` of a function's call of values is copied as one value is

Status: Accepted. Amends [0755](0755-a-shared-return-is-each-breaks.md)
and [0754](0754-a-return-arms-go-on-to-is-in-each.md), whose `return`
copied into each `break` or path was of one value, a constant or a name
(`returns_one_value`): a call of such, `f(a, 1)`, too. Completes
[0944](0944-a-label-the-program-wrote-labels-the-switch.md) for React's
getTag as it's written.

Case ([0262](0262-when-rust-and-js-disagree.md)): N. Each path runs the
call once, where it went to the one `return`; the call's arguments are
names and constants, which read the same wherever it's put.

## Context

React's `createFiberFromTypeAndProps` returns a call from each guarded arm
that falls to the next:

```js
case REACT_SCOPE_TYPE:
  if (enableScopeAPI) {
    return createFiberFromScope(type, pendingProps, mode, lanes, key);
  }
// Fall through
```

MIR shares each arm's body among the alternatives that reach it, so the
JS had blocks MIR made, `bb127: { .. break bb127; .. } return
createFiberFromScope(..);`, and `jumps past blocks` and `returns into
branches` copied only a `return` of one value into each way there, React's
literal returns of the corpus's test: the `match` stayed an `if` chain of
`break bb53`, `bb127`, `bb136` and `bb301`.

## Decision

**A `return` of a call of a function's name, each argument a constant or a
name, is copied as one value is** (`returns_one_value`): into each `break`
of the block before it, and each path of the `if` that goes on to it. The
`match` is then one `getTag: switch (type) { .. }` of falling cases, each
case's `return` written once. A call of anything else, `complex(array.length)`,
stays where it's written, once.

## What holds it

- `test/lowering-labels.test.ts`, "guarded arms that return calls fall
  through in one labeled switch", React's whole getTag, run.
- Mutations: `returned-call-not-one-value`, `return-into-branches-of-any-value`.
