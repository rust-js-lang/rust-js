# 0499. `retain_mut` drops what it removes

Status: Accepted. Extends [0098](0098-destructors.md).

Case: C ([0262](0262-when-rust-and-js-disagree.md)): what's dropped, and
when, as std's.

## Context

`retain` of a `Vec` of values with a destructor drops each it removes,
`$retain` given the items' drop (ADR 0098); `retain_mut`, the same but for
a `&mut` to each, was refused.

## Decision

**`retain_mut` drops each item it doesn't keep as it goes, `$retainMut`
given the items' drop, as `retain` does, of a `Vec` and a `VecDeque`.**

## Why

- **It's std's**: `retain_mut_drops` changes and keeps some items with a
  destructor, of a `Vec` and a `VecDeque`, and numbers through handles, with
  native Rust.

## Consequences

- **`splice`, and `VecDeque::from` of a `Vec`, of values with a
  destructor**, are refused yet. (Amended: neither is, ADRs 0500, 0501.)

## Amendment: a `VecDeque`'s is std's own (2026-11-08, #909)

The decision above had a `VecDeque`'s `retain_mut` drop each item it
doesn't keep as it goes, which is a `Vec`'s algorithm, and its premise was
wrong for a deque. std's `VecDeque::retain_mut` swaps each item it keeps
forward and truncates the rest after the last call, so a closure that
prints sees each drop after every call. Its `retain` is its `retain_mut`.
And with no guard, a panicking closure leaves every item in the deque, in
the order the swaps left them. Case C ([0262](0262-when-rust-and-js-disagree.md)):
results change, so Rust's behaviour is kept. Kind: wrong decision.

**A `VecDeque`'s `retain` and `retain_mut` are `$retainDeque(q, f,
handles, drop)`**, std's two stages and `truncate`, the items past those
kept dropped as `truncate` drops them. A `Vec`'s is as decided above.

Held by `test/corpus/vec_deque_retain_swaps.rs`, both methods and a
`Vec`'s `retain` under a closure that panics, against native Rust in both
builds, and the mutations `deque-retain-*` and `retain-deque-*`. The unwind
table's 20 case-builds of #909 pass.
