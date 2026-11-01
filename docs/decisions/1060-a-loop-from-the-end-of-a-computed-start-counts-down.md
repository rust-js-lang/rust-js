# 1060. A loop from the end down to a computed start counts down

Status: Accepted (rd529, P2, 2026-11-02). Amends
[0904](0904-a-loop-from-the-end-counts-down.md), whose range's start was a
constant or a variable.

Case ([0262](0262-when-rust-and-js-disagree.md)): N, no observable
difference: the start is read at each step only where it reads the same
at each, as Rust's, read once.

## Context

React's Flight client's flushComponentPerformance walks the entries after
one from the end, `for (let j = endTimeIdx - 1; j > i; j--)`, twice. The
Rust, `for j in (i + 1..end_time_idx).rev()`, printed `for (const j of
$iterator({ start: i + 1, end }, $rangeNextBack))`: ADR 0904 counted down
only from a start that's a constant or a variable, as the `for`'s test
reads it at each step.

## Decision

- **A start that reads the same at each step counts down too**: one of
  constants, variables the body doesn't assign, and arithmetic of them,
  `i + 1` or `(i + 1) >>> 0`; a sum of numbers is a number, as JS's `--`
  needs. Not a length, `items.len() - 2`, which the body may change, nor
  a variable it assigns: those are stepped by the iterator as before.
- **Down to one past another, `j > i`**: of a start `i + 1`, which JS adds
  exactly, a step's integer is at least `i + 1` where it's more than `i`.
  One Rust wraps, `(i + 1) >>> 0`, stays `j >= (i + 1) >>> 0`, as
  `u32::MAX`'s wraps to `0`.

## Why

- Upstream's loop, and the one a person writes; the test reads what Rust
  read once, the same at each step.

## Alternatives

- **The start held in a `const` before the loop**: also correct, and
  where it reads the same at each step one name more than upstream's.

## Consequences

- Held by the corpus's `range_rev_from_computed`, against native Rust:
  `after`'s `j > i`, `wrapped`'s `u32::MAX` start, a start the body
  assigns and a length it grows, each stepped as Rust's. Mutations
  `counted-down-start-of-sums-unread`, `counted-down-start-assigned`,
  `counted-down-sum-not-a-number`, `counted-down-past-one-kept`
  (`tidy/loops.rs`).
