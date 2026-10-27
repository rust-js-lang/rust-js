# 0981. A held struct's `rfold` is std's compiled one where it reaches the crate's own

Status: Accepted, under the user's goal of 2026-10-26: the plan "rust-js:
one representation per iterator", its third phase, a runtime helper that
stood in for a std body retired.

Case ([0262](0262-when-rust-and-js-disagree.md)): none, no observable
difference. std's compiled `rfold` gives what `$fuseRfold` gave of a
`Fuse`, and of another adapter now what std's gives.

## Context

A std struct held and made knowing its back (`$knowing`) was given an
`rfold` only where it was a `Fuse`: `$fuseRfold`, a hand-written copy of
std's `Fuse::rfold` (#224). The comment said the compiled one was generic
over what it folds. `compiled_fold` has given compiled folds their `B`
and `F` since ADR 0912. Other adapters, a held `Map` of the crate's own
iterator, were folded from their back by `next_back` where std passes the
crate's own `rfold` on.

## Decision

A held std struct's `rfold` is std's compiled one (`compiled_fold`)
wherever its `rfold` reaches the `rfold` the crate's own iterator writes
(`rfolds_its_own`, the rule of #220 and #224). Of another, folding by
`next_back` gives the same, and none is given. `$fuseRfold` is removed.

`compiled_step` resolves a method with its own generic parameters,
`rfold`'s `B` and `F`, as themselves, as `type_facts` does (ADR 0975).
Before, a method with its own generics resolved with too few arguments,
a rustc internal error.

Two of the runtime's helpers that stand next to std bodies stay, as
rewrites' helpers, not copies of a compiled body:

- `$revAdvanceBy`: a `Rev`'s `advance_by` of a `Vec`'s `IntoIter`, whose
  `advance_back_by` std drops as one slice, in forward order, where
  stepping `next_back` drops them in reverse (#286).
- `$advanceByOf`: the recognizers' `$skipping`'s.

## Consequences

Six normal snapshots change, by four lines. The mutation
`knowing-rfold-unasked` replaces `knowing-fuse-rfold-unasked`, and
`fuse-rfold-not-own` goes with `$fuseRfold`.
