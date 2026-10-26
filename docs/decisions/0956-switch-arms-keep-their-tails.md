# 0956. The arms of a `switch` keep their statements, a tail they share too

Status: Accepted. Extends [0629](0629-a-match-of-constants-is-a-switch.md):
a `match` of constants is a `switch` of its arms as written, which tidy's
`shared tails` no longer takes apart first.

Case ([0262](0262-when-rust-and-js-disagree.md)): N. Where a statement is
written, in each case or once after them, runs it on the same paths.

## Context

React's `commitAttachRef` is one `switch`, a case per tag; its Fragment
case and its `default` each end `instanceToUse = finishedWork.stateNode`.
`shared tails`, which writes the statements both branches of an `if` end
with once after it, ran on MIR's chain of the `match`'s tests before
`switches` read it: the shared tail moved out of the last `else`, and the
JS was `if (..) {..} else if (..) {..} else { switch (tag) { case
FRAGMENT: .. } instanceToUse = finishedWork.stateNode; }`.

## Decision

**`shared tails` leaves an `if` of a test of a `match` of constants'
arm** (`switches::switched`, the one answer of which `if`s are a
`switch`'s), so each arm is its case, as written.

## What holds it

- `test/lowering-switches.test.ts`, "arms of a switch that end alike are
  each their case's".
- Mutation: `switch-arms-tail-shared`.
