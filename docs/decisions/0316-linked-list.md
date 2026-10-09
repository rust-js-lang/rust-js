# 0316. A `LinkedList` is an array, as a `VecDeque` is

Status: Accepted. Extends [0068](0068-queues.md); counted by
[0314](0314-std-data-structures.md).

Case: A ([0262](0262-when-rust-and-js-disagree.md)).

## Context

`LinkedList` was refused: its nodes are raw pointers, which rust-js has no
JS for. ADR 0314's count had it at 0 of 20.

## Decision

**A `LinkedList` is a JS array, as a `VecDeque` is, and its methods are a
deque's: `push_front` is `unshift`, `pop_back` is `pop`, `front` is
`list[0]`, `append`, `split_off`, `contains`, `iter`, `extend` and
`collect` as a `Vec`'s.**

## Why

- **It's the same program**: what a list can be asked, its items in
  order, its ends, its length, is what an array answers; only how fast
  differs, as a deque's does (ADR 0068).
- **It's the JavaScript a person writes**: an array, `unshift` and `pop`.
- **It's tested**: a corpus case pushes and pops at both ends, appends,
  splits, sums, collects, extends and loops, against native Rust;
  mutations take it back to its pointers, and its ends from a deque's.
- `docs/std-coverage.txt`: `LinkedList` 15 of 20; its cursors are
  unstable.
