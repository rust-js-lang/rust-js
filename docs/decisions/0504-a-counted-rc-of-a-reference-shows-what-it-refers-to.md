# 0504. A counted `Rc` of a reference shows what it refers to

Status: Accepted. Extends [0320](0320-counted-rcs.md).

Case: N ([0262](0262-when-rust-and-js-disagree.md)): the same text.

## Context

`{}` and `{:?}` of a counted `Rc` show what it points at, its `value`
(ADR 0320). Of an `Rc<&T>`, that's a reference, which was left: `{}` of a
`&i32` was refused.

## Decision

**What's shown through a counted `Rc` is through the references it holds
too, as of any value: `through_boxes` peels them after the `Rc`'s
`value`.**

## Why

- **It's std's**: `counted_rc_borrow` shows a counted `Rc<&i32>` and its
  clone, with native Rust.
