# 0503. A counted `Rc`'s `borrow()` is its value

Status: Accepted. Extends [0320](0320-counted-rcs.md). Amended by
[0505](0505-an-rcs-as-ref-is-what-it-points-at.md): `as_ref()` too.

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

## Amendment: generic code's `Borrow` and `AsRef` of a counted `Rc` (2026-11-12, #1503)

Case: C, a wrong answer fixed. A known type's `rc.borrow()` was its value,
but the dictionary generic code is given for an `Rc`'s `Borrow<Q>`,
`BorrowMut<Q>` or `AsRef<U>` was the identity, `{ borrow: (value) => value
}`, right of an `Rc` that's its value and wrong of a counted one: `fn
len<K: Borrow<str>>(k: K) -> usize { k.borrow().len() }` of a counted
`Rc<str>` was 0 where native Rust gives 3, and `T: AsRef<str>`'s
`t.as_ref().to_uppercase()` threw. Each entry now goes through the type's
references, `Box`es and `Rc`s to what it borrows as, `through_boxes`'s (the
one owner of what a counted `Rc` points at, by `counted_rc`), so a counted
one's is its `value`, `{ borrow: (value) => value.value }`; one not counted
is its value already. A dictionary is of its type, so a counted `Rc<str>`'s
and an uncounted `Rc<[i32]>`'s differ in one program.

Proof: corpus case generic_borrow_of_a_counted_rc, a counted `Rc<str>`,
`Rc<[i32]>` and `Rc<i32>` given generic code's `Borrow` and `AsRef`, a
`Box` of one, and an uncounted `Rc<[i32]>`, against native Rust in both
builds. Mutations `as-ref-of-a-counted-rc-is-the-rc` and
`borrow-of-a-counted-rc-is-the-rc`, caught.

## Amendment: as far as what it's asked as (2026-11-12, #1518)

Case: C, a wrong answer of #1503's fix. It read through every `Box` and
counted `Rc` of the implementing type, whatever the trait's target: a
`Box<Rc<str>>` given as `T: AsRef<Rc<str>>` or `K: Borrow<Rc<str>>` was
read to its text, where what it's asked as is the counted `Rc`, so
`Rc::strong_count(t.as_ref())` was undefined and `k.borrow().len()` threw.
How far to read is the pair of types: `through_boxes_to(value, ty, to)`,
`through_boxes`' one loop, stops once what's left is the target, `to`, so a
`Box<Rc<str>>`'s `AsRef<Rc<str>>` is the `Rc` itself and an `Rc<str>`'s
`AsRef<str>` what it points at, as before.

Proof: corpus case generic_borrow_of_a_counted_rc, a `Box<Rc<str>>` as
`AsRef<Rc<str>>` (its count read) and `Borrow<Rc<str>>`, beside #1503's
`Rc<str>` as `AsRef<str>`, against native Rust in both builds. Mutation
`borrowed-past-its-target`, caught; #1503's two refitted.
