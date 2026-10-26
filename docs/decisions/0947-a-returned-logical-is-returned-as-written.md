# 0947. A `return` of a logical the program wrote is that `return`, wherever its tests' `false` goes on

Status: Accepted. Extends [0613](0613-and-or-mixed-are-returned-as-written.md),
whose `logical_returns` folded `if (t) { return a; } return b;` only where
neither `return` was the program's and both were in one list.

Case ([0262](0262-when-rust-and-js-disagree.md)): N. The value is the
one the branches returned, each test run where and in the order it ran.

## Context

React's `isLegacyActEnvironment`:

```js
if (__DEV__) {
  const isReactActEnvironmentGlobal = ..;
  return warnsIfNotActing && isReactActEnvironmentGlobal && ..;
}
return false;
```

MIR makes the `&&` chain's tests `if`s, each failing test jumping to the
`return false` after the `if (__DEV__)`, which tidy shared: `if (a) { if
(b) { return c; } }`, no `return` after them in their list to fold. And
`return a || !b;` was two `return`s, `if (a) { return true; } return
!b;`: both inside the `return` the program wrote, which `logical_returns`
took for two it wrote.

## Decision

- **A chain of `if`s with no `else`, each the one statement of the one
  before, the last's a `return v`, at the end of a list that goes on to a
  `return d`** (after it, or after the `if` whose branch the list is),
  each test of a logical inside a `return` the program wrote, is `return
  t1 && .. && v` (`returned_logicals`), folded as `conditional` folds one.
- **A `return` inside the `return` the program wrote around the test**
  is that logical's (`returned_as_written`), so `logical_returns` folds
  `return a || !b;` as written.

## What holds it

- `test/lowering-written-branches.test.ts`, "a returned && chain in an
  if is returned as written".
- Mutations: `returned-logicals-*`.
