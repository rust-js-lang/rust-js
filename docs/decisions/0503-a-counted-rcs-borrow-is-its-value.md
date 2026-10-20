# 0503. A counted `Rc`'s `borrow()` is its value

Status: Accepted. Extends [0320](0320-counted-rcs.md).

Case: C ([0262](0262-when-rust-and-js-disagree.md)): the value Rust's
`Borrow` gives.

## Context

A counted `Rc` is `{ value, strong, weak }` (ADR 0320), and `*rc` is its
`value`. `rc.borrow()`, std's `Borrow<T> for Rc<T>`, was written as the
value itself, as of an uncounted `Rc`: of a counted one, the `Rc`, shown
`[object Object]`.

## Decision

**`rc.borrow()` of an `Rc<T>` to `T` is what it points at, as `*rc` is,
`rc_same`'s: of a counted one, its `value`.**

## Why

- **It's std's**: `counted_rc_borrow` borrows a counted `Rc<String>` and
  shows it, with native Rust.

## Consequences

- **`rc.as_ref()`** of an `Rc` is refused yet, its `AsRef` evidence
  unmade.
