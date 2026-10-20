# 0519. A `Cell` or a `OnceCell` of what has a destructor

Status: Accepted. Amends [0320](0320-counted-rcs.md).

Case: C ([0262](0262-when-rust-and-js-disagree.md)): what drops would
change, so Rust's is kept.

## Context

A `Cell` or a `OnceCell` of a value with a destructor was refused, by
type: a `Cell` drops the old value when it's set, which rust-js didn't.
Both are `{ value }` already, as a `RefCell` is, whose value drops with it.

## Decision

**A `Cell` drops as its value does, and its `set` drops the old value once
the new is in its place, as std's `replace`s then drops it. A `OnceCell`
drops as its `Option`, its `{ value }`, `undefined` until it's set; its
`set`, `get_or_init`, `take` and the rest drop nothing.** A `LazyCell` stays
refused: what it drops is its function or its value, by its state.

## Why

- **It's the same program**: `cell_drops` sets, replaces, takes and drops
  a `Cell` of `Option`s, and sets, sets again, inits and takes `OnceCell`s,
  against native Rust.

## Consequences

- The diagnostic and the mutations that held a `Cell`'s refusal hold a
  `LazyCell`'s.
- Mutations drop no old value, drop no held `Cell` or `OnceCell`, and
  refuse each.
