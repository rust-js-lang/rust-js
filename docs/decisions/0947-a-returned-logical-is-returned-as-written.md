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

## Amendment: a function's last value of `||`s (2026-10-27)

React's `isFragmentInstanceHostBoundary` returns `tag === HostComponent ||
tag === HostRoot || (supportsSingletons ? tag === HostSingleton : false)`,
its port's last expression, no `return` written: MIR's `if (a) { if (b) {
return c; } } return true;` stayed nested `if`s, as `logical_returns`
folds one `if` alone. **A chain of `if`s followed in its list by the
`return` its failing tests go on to, neither the program's, each test of
a logical the value is a part of too, is one `return`**, folded as before.
Not an `if (a || b) { return x; } return y;`, whose `x` is outside its
test's logical, which stays as written. Held by
`test/lowering-written-branches.test.ts`, "a function's last || of an
if's value is one return", and mutations `last-or-chain-unfolded` and
`last-or-chain-of-an-if-test`.

## Amendment: no check of where the value is

The fold checked that the value returned is inside its tests' logical,
so `if (a || b) { return x; } return y;` stays as written. It never
decides: of an `x` that isn't `true` or `false`, the fold is a `?:`,
which it refuses; of one that is, `if a || b { true } else { y }`, an
earlier pass makes the value, `!a && !b ? y : true`. With a probe that
panicked where it decided and the fold went on, no corpus case, lowering,
JSX, snapshot, port, guide, std or crate test reached it, and
`last-or-chain-of-an-if-test` survived every run. The check and its
mutation went; `last-or-chain-unfolded` holds the fold.

## Amendment: a last value of `||`s is a written `if`'s now

Since a written `if` or `match` of `bool`s with a literal branch is the
`&&` or `||` it means (ADR 0879, amended, 22116187d), the
`isFragmentInstanceHostBoundary` form, a function's last `||`s with no
`return` written, is that `||` before this pass sees it. The fold of a
chain no written `return` is around, `last-or-chain-unfolded`'s, then
changed no JS of the corpus (with its snapshots) or of the lowering,
snapshot, JSX, port, compiler and tidy-stage suites, and its test, "a
function's last || of an if's value is one return", passes without it.
That arm and its mutation went: the fold is of a chain whose tests a
written `return` is around, as before 2026-10-27.
