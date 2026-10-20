# 0509. More of std's calls of values with a destructor

Status: Accepted. Extends [0098](0098-destructors.md) and
[0179](0179-option-filter-map-or-drops.md).

Case: C ([0262](0262-when-rust-and-js-disagree.md)): what's dropped, and
when, would change, so Rust's is kept.

## Context

Probing std's calls of values with a destructor found these refused,
though none drops anything std doesn't, or each drops one thing a known
way: an array's `map`, `Option::unwrap_or_default` and `as_mut`,
`Result::unwrap_or_default`, `Vec::pop_if`, `VecDeque::pop_front_if`, and
`shrink_to_fit`, `shrink_to` and `reserve`.

And `or_dropped`, which drops the variant a call gives up, dropped it
before making what the call gives otherwise. Std's drops it when the call
returns, after: `Result::unwrap_or_default`'s `Err` once its default is
made, which a `Default` that prints shows.

## Decision

**Each is allowed: a capacity's change, `pop_if`, `pop_front_if`, an
array's `map` and `as_mut` drop nothing; `Result::unwrap_or_default` drops
its `Err` once the default's made, by `kept_or`, as `and` and `or` drop
what they give up. And `or_dropped` makes what it gives otherwise first,
then drops what's given up, for every call that asks it.**

## Why

- **It's the same program**: `default_and_pop_if_drops` runs each, with a
  `Default` that prints, against native Rust.
- **One order**: every call that gives up a variant drops it last, as
  std's does when it returns; what the others give otherwise, `false` or
  `undefined`, can't show the difference, so their output only moves the
  drop after the assignment.

## Consequences

- `unused_value_drops`, `and_value_drops` and `combinators_owned_drops`
  drop after they assign.
- The diagnostic and the mutation that hold `check_takes_drops` refuse
  `iter::successors` of values with a destructor now.
- Mutations drop first, keep the `Err`, and refuse each call.
