# 0954. A call that never returns ends what it's in

Status: Accepted.

Case ([0262](0262-when-rust-and-js-disagree.md)): N. What follows a call
that never returns is never run.

## Context

React's `tryToClaimNextHydratableFormMarkerInstance` ends with
`throwOnHydrationMismatch(fiber);`, which throws. Its port's Rust ends its
`-> bool` function with the `-> !` call, and the JS read
`throwOnHydrationMismatch(id); let result; return result;`: MIR's call has
no block to return to, so the JS went on past it, to the function's
`return` of a value no path made.

## Decision

**A call MIR says never returns, of no block to go on to, is
`Known::Diverges`**, which `js::ends` takes as a statement that leaves,
as a `throw` is: tidy's passes then leave out what's after it, the
`let result; return result;` with it.

## What holds it

- `test/lowering-written-branches.test.ts`, "a call that never returns
  ends its function".
- Mutation: `diverging-call-goes-on`.
