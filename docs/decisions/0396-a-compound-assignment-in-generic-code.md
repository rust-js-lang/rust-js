# 0396. A compound assignment in generic code is given its dictionary

Status: Accepted. Extends [0108](0108-generic-operators-and-into.md) and
[0180](0180-generic-writers.md).

Case: C ([0262](0262-when-rust-and-js-disagree.md)): the value, and the
order each side runs in, are Rust's.

## Context

`a += b` of a `T: AddAssign`, and of `SubAssign` and the rest, was
refused in generic code, though `a + b` of a `T: Add` had a dictionary
since ADR 0108. And `w.0 += b` of the crate's own type, through a place,
was refused too: lowering took any `+=` through a place for a number's.
rustc's `expr/compound-assignment/eval-order.rs` failed on both.

## Decision

**A `T: AddAssign` and its kin are given a dictionary of the operator's
method, which takes its operand in the box a generic `&mut` is given in
(ADR 0180): std's of numbers, and a `String`'s `+= &str`, assign the
box's `value`; the crate's own is its impl's.**

```js
function total(items, TAddAssign, TDefault) {
  let t = TDefault.default();
  for (const x of items) {
    const self = { value: t };
    TAddAssign.add_assign(self, x);
    t = self.value;
  }
  return t;
}
total([1, 2, 3], { add_assign: (cell, b) => { cell.value = (cell.value + b) | 0; } }, ..);
```

- **A `+=` through a place is a number's or a string's only**; another's
  is its impl's call, or its dictionary's.
- **Where the type is known, nothing changes**: a known operator's
  assignment is in place, as `a + b`'s and `x.into()`'s are (ADR 0108).
- **Each side runs in Rust's order**: an overloaded `+=`'s left side
  first, as MIR has it, a primitive's right side first.
- **Only generic code reads an operator's dictionary, or a `Deref`'s** (ADR
  0394): a crate's impl's is written only where it's reached,
  `read_only_generically`'s one answer. The unread ones of `a + b`'s that
  every operator impl had are gone.

## Why

- **It's the program Rust runs**: the corpus's
  `generic_compound_assign.rs` sums numbers and the crate's money, scales,
  shifts and appends text in generic code, and checks each side's order,
  with native Rust; the rustc test passes.

## Amendment: the crate's own `*Assign` of a number

`a += b` of a number or a `String` was the place assigned by the type's
own operator wherever `a` was one, so the crate's `impl AddAssign<Cents>
for i32` was skipped: `t += Cents(5)` was `t + [5]` (#153). Whether it's
std's is `resolves_to_crate`'s answer, as of every other call: the
crate's impl is called, of the place's box. Held by
`test/corpus/assign_impl_on_number.rs`.
