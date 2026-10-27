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

## Amendment: an arm that goes on to its value keeps it, `_`'s its `default` (2026-10-27)

Case ([0262](0262-when-rust-and-js-disagree.md)): N, the same `return`
runs on the same paths.

React's unwindWork ends `case SuspenseListComponent: { ..; if (..) { .. }
return null; } default: return null;`; its port, an arm `{ if .. { .. }
None }` and `_ => None` (c22 of the port's gaps). The arm's `if` goes on to
its `None`, and `shared tails`, of an `if` without an `else` whose branch
ends as what follows it does, `if (c) { a; return v; } return v;`, took
the arm's away: `case 25: if (..) { .. } } return null;`, `_`'s value
after the `switch`.

**That `if` keeps its tail where it tests a `match` of constants' arm**
(`switches::switched`), as the one with an `else` does: `case 25: if (..)
{ .. } return null; default: return null;`, as written.

Where a case is braced is unchanged: where it declares, as JS needs it
not to leak its names into the next case's; React's braces, which follow
no rule its source keeps (`case HostComponent: { .. }`, `case
ContextProvider: const context = ..;`), have no Rust to read them from.

Held by `test/lowering-switches.test.ts`, "an arm of a switch that goes on
to its value returns it, and `_`'s is its default", run; the mutation
`switch-arm-going-on-tail-shared` (`mir/tidy/branches.ts`).
