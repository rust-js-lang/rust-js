# 0927. A held `Skip` and `Take` fold from their back by std's own `rfold`, and a std body is given what the std body it calls steps

Status: Accepted, under the user's goal of 2026-10-26 ("keep going until
all done, the choice is your recommendation"). The plan "rust-js: one
representation per iterator", its second phase. Extends
[0913](0913-std-s-generic-try-is-a-dictionary.md) and
[0487](0487-an-iterator-given-as-it-is-has-its-extensions-dictionaries.md).

Case: C ([0262](0262-when-rust-and-js-disagree.md)): results change.
With std's structs alone, `count().skip(2).map(f).rfold(..)` of the
crate's iterator with a destructor never dropped it: std's `Skip::rfold`
wasn't compiled, so what folded it from its back didn't take it.

## Context

std's `Skip` and `Take` write their own `try_rfold`, and `rfold` by it
(`impl_fold_via_try_fold!`). Compiling them, `Skip::rfold`'s
`self.try_rfold(..)` asked `I`'s `DoubleEndedIterator` dictionary, which
`rfold` wasn't given: a body is given an iterator bound's dictionary only
where it steps its parameter by it (ADR 0487), and the analysis took what
a compiled std struct's method steps from its manifest, `foreign.item(..)`,
which std's own bodies, being compiled, have none of.

## Decision

- **A held `skip()` and `take()` fold from their back by std's compiled
  `rfold` and `try_rfold`.**
- **A std body calling another one being compiled passes its parameters
  on to it** (`stepped_params`), as one calling the crate's own does: it
  steps them as that one does.

## Consequences

- ADR 0908's `differs` is 19, from 21: `fold_back_drops_what_it_takes.rs`
  and `rev_in_place_through_adapters.rs` run as native Rust does with
  std's structs alone.
- std's `Skip` and `Take` are compiled whole (`docs/std-coverage.txt`: 11
  of 11, 12 of 12). The normal build's JS doesn't change.
- Mutations: `std-body-callee-steps-unpassed`.
