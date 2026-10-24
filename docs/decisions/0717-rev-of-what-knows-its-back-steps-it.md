# 0717. `rev()` of what's made knowing its back steps it from there

Status: Accepted. Amends [0535](0535-a-lazy-map-steps-from-its-back.md),
whose `rev()` of a lazy chain stepped it from its back, but not of one
JS iterates as it is.

Case ([0262](0262-when-rust-and-js-disagree.md)): R, results change.

## Context

`Down { n: 3 }.chain(vec![7, 8]).rev().collect()` of the crate's own
iterator, whose `next_back` gives other items than its `next`, printed
`[8, 7, 0, 1, 2]` where Rust prints `[8, 7, 102, 101, 100]`. The chain
was made knowing its back, `$chainEnds(..)`, but `rev_items` took
`Iterator.from(..)` of it for items it could step from the front and
reverse, `Array.from(..).toReversed()`, which is only Rust's where what's
stepped from the back is what's stepped from the front reversed.

Triaging the check's surviving mutations after the merge of 2026-10-24
found it, and what the merge left no program reaches.

## Decision

- **What's made knowing its back (`knows_its_back`) is `$rev` of it**,
  its own back stepped as Rust's `Rev` does. Only what isn't is its
  items reversed.
- **What no program reaches is gone, or its mutation is**:
  - `as_double_ended`'s arm of an iterator with its own `next_back`:
    `own_next_back` answers only of the crate's iterators, which
    `user_iterator_knowing` makes first;
  - `$chunksExactMut` in what's made an array, which `steps_an_array`
    answers by its type;
  - `as_js_iterator`'s reveal of an `impl Iterator`, which each caller
    gives revealed;
  - the mutations `mir-fold-out-of-order`, of `admit`'s check that what
    acts is read in the order it's made, kept as a safeguard: a struct's
    fields are written as the program writes them, so MIR's operands come
    in the order they're made; and `thread-local-with-taken-plain`,
    whose `with` taken as a read changes no program, a closure given it
    being `quiet` or not as a `with_borrow`'s is.
- The other survivors were tests that no longer reach what they hold:
  each widened (`held_enumerate.rs`'s `next`, `adapter_source_drops.rs`'s
  chain, whose first part is dropped before its second is stepped, as
  `Chain` drops it) or retargeted to a case whose JS the mutation changes.

## Consequences

- Held by `test/corpus/chain_rev_own_back.rs` against native Rust, and
  the mutation `rev-of-own-back-reversed`.
