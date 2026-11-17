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

## Amendment (2026-11-18, #1577): a counted `Rc`'s `&mut` cell is read through

The amendment above let the loop give back the reference a counted `Rc`
holds, for the reference's own formatting to peel. That holds of a `&T`,
whose JS is what it refers to, but not of a `&mut` that's a cell, a
number's or a `String`'s, whose `value` is what it refers to: `{}` of a
counted `Rc<&mut u32>` was refused, "`{}` of a `&mut u32`". It showed
only where the crate counts every `Rc`: beside an impl for any `Rc<T>`
and an `Rc<dyn Fn>` whose `ptr_eq` it asks (ADRs 0852, 0959), the
neighbour shuffle's `boxed_mut_ref_shown.rs` beside
`rc_of_an_impls_parameter_counted_as_any.rs` or `rc_of_impl_fn_returned.rs`.
`through_boxes_until` now goes through the references a counted `Rc`'s
pointee holds, `through_refs_until`, as it does an uncounted one's: a
cell's `value` read, a `&T` passed as it is, so nothing is peeled twice.

Case C of ADR 0262: a refusal where native Rust shows the value. Corpus
case `counted_rc_of_a_mut_ref_shown` puts the neighbour's two ingredients
beside `boxed_mut_ref_shown`'s rows: an `Rc` and a `Box` of a `&mut u32`,
one written through `Rc::get_mut` before it's shown, `**through += 10`, a
`&mut String`'s and a counted `&mut Loud`'s. It runs against native in
both builds. Both pairs run as native too, and leave
test/neighbours-known.txt. Mutation `counted-rc-pointee-not-through-refs`
is caught.
