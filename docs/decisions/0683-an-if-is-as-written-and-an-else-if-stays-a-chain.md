# 0683. An `if` keeps its test and branches as written, and an `else if` stays a chain

Status: Accepted. Amends [0237](0237-no-else-after-leaving.md), whose
`else` after a branch that leaves goes only where each arm before it
leaves too; extends [0575](0575-an-if-keeps-its-else-and-a-tail-if-is-a-conditional.md)
and [0364](0364-mir-lowering.md)'s tidy.

Case ([0262](0262-when-rust-and-js-disagree.md)): N. Each rewrite changes
how the JS reads, not what runs.

## Context

React's `describeObjectForErrorMessage` (shared/ReactSerializationErrors.js),
ported, read worse than its original twice:

```js
// The original:                          // rust-js's:
if (__DEV__ && jsxChildrenParents.has(o)) {   if (!__DEV__ || !jsxChildrenParents.has(o)) {
  // Print JSX Children                         // Print Array
  ..                                            ..
} else {                                      } else {
  // Print Array                                // Print JSX Children
  ..                                            ..
}                                             }

if (o.$$typeof === REACT_ELEMENT_TYPE) {      if (o.$$typeof === REACT_ELEMENT_TYPE) {
  str = ..;                                     str = ..;
} else if (o.$$typeof === CLIENT_REF) {       } else {
  return describeClientReference(o);            if (o.$$typeof === CLIENT_REF) {
} else if (__DEV__ && ..) {                       return describeClientReference(o);
  ..                                            }
} else {                                        if (__DEV__ && ..) { .. } else { .. }
  ..                                          }
}
```

MIR jumps from each test of an `&&` to the `else`, so the `then` is the
labeled block's and the `else` follows it: `l: { if (a) { if (b) { s;
break l; } } t }`. Tidy wrote that `if (!(a && b)) { t } else { s }`,
counting on a later pass to turn it back, which none does where a branch
ends in a loop. And preparation's rule that an `if` whose branch leaves
has no `else` (ADR 0237) took the `else` of an `else if` too, whose
statements were then the outer `else`'s: the chain broken.

## Decision

**An `if` of an `&&` MIR made a block of is as the program wrote it**:
`if (a && b) { s } else { t }` where `s` is written first, `if (a || b) {
t } else { s }`, its tests negated, where `t` is. Which is first is where
it's written (`first_written`), as a conditional's branches already are.

**An `else if` of an arm that goes on stays an `else if`**: `if (a) { x =
1; } else if (b) { return; } else { .. }` keeps its chain. An arm that
leaves loses its `else` only where every arm before it leaves too, the
chain's `if`s one after the other, as before: what ESLint's
`no-else-return` asks, which allows an `else if` after an arm that goes
on.

## Why

- **It's how it's written**, in the Rust and in the original JS: React's
  own `&&` test first, its `else if` chain of four arms.
- **One rule of which branch is first**: by where the program wrote it,
  as `branch_values` orders a conditional's and `written_elses` an `if`
  MIR made a guard of; not a negation a later pass must undo.
- **A chain read as one**: breaking it puts an arm's `if` in an `else`
  beside the next arm's, which reads as nested code, not a choice.

## Alternatives

- **Turn `if (!a || !b) { t } else { s }` back in a later pass**: the
  turn and its undoing, two rules for one question. Rejected.
- **Keep every `else` the program wrote, after a `return` too**: React and
  ESLint write `if (c) { return a; } return b;`, and ADR 0237's reasons
  hold. Rejected.

## Consequences

- No corpus case's or snapshot's JS changed: none had either shape.
- Preparation's `unelsed` holds the `else` lists of a kept chain as it
  walks the lists from the outside in.

## What holds it

- `test/lowering-written-branches.test.ts`, "an if of an && keeps its test
  and its branches' order, each a loop", and "an else-if chain keeps its
  else past an arm that returns", with a chain each of whose arms
  returns, unchanged; each run.
- Mutations: `mir-tidy-and-chain-turned`, `else-if-unelsed`,
  `else-if-links-unheld`.
