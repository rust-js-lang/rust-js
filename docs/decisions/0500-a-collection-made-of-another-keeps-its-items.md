# 0500. A collection made of another keeps its items

Status: Accepted. Extends [0098](0098-destructors.md) and
[0052](0052-std-trait-impls.md).

Case: C ([0262](0262-when-rust-and-js-disagree.md)): what's dropped, and
when, as std's.

## Context

`VecDeque::from(v)`, `Vec::from([..])` and `LinkedList::from([..])` are a
copy of what they're given (ADR 0052), which they keep, each item; of
values with a destructor, they were refused as taking what they might
drop.

## Decision

**A collection made of another it's given, `Std::ToVec`, keeps each item,
which it drops as it's dropped.** A `to_vec()` of a borrowed slice takes
nothing it'd drop, as before.

## Why

- **It's std's**: `collection_from_drops` makes a `VecDeque`, a `Vec` and a
  `LinkedList` of values with a destructor, pops one and drops the rest,
  with native Rust.
