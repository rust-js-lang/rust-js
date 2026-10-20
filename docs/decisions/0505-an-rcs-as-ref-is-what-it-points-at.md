# 0505. An `Rc`'s `as_ref()` is what it points at

Status: Accepted. Amends [0503](0503-a-counted-rcs-borrow-is-its-value.md),
extends [0331](0331-dyn-any.md).

Case: N ([0262](0262-when-rust-and-js-disagree.md)): the same value.

## Context

`rc.as_ref()`, std's `AsRef<T> for Rc<T>`, was refused: rust-js made no
evidence for it (ADR 0503's consequence).

## Decision

**`rc.as_ref()` is what the `Rc` points at, as `rc.borrow()` and `*rc`
are, `rc_same`'s: the value itself of an uncounted one, its `value` of a
counted one.**

## Why

- **It's std's**: `counted_rc_borrow` takes `as_ref()` of a counted and an
  uncounted `Rc`, with native Rust.
