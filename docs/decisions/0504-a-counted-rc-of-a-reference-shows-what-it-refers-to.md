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

## Amendment: a `Box`'s and an uncounted `Rc`'s too

A `Box` or an `Rc` nothing counts is what it holds in JS (ADR 0023), and
`shown_type` showed it as that type, its references peeled, but its value
stayed the `&mut`'s handle: `{}` and `{:?}` of a `Box<&mut u32>` or an
`Rc<&mut u32>` were `[object Object]` (#197). `through_boxes`, the one
that takes a value and its type through what holds it, goes through
these too, then through the references they hold, as it does a counted
`Rc`'s: `boxed.value`. `boxed_mut_ref_shown` holds both, a counted `Rc`'s
too, with `box-of-mut-ref-shown-as-handle` and `rc-of-ref-unpeeled`.

## Amendment: one place peels what an `Rc` holds

`through_boxes` peeled the references a counted `Rc` holds, an
`Rc<&mut T>`'s handle (#197), but `{}` and `{:?}` of a `&T` already show
it through its referent, so the JS was the same without it: the
nightly's mutation `rc-of-ref-unpeeled` survived (#228). The loop gives
back the reference it reaches, and the reference's own formatting peels
it.
