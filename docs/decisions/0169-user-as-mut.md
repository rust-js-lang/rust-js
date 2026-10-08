# 0169. A type's own `AsMut` and `BorrowMut`, called on the type

Status: Accepted. Extends [0162](0162-generic-as-ref.md) and
[0167](0167-borrow.md).

Case: N, D ([0262](0262-when-rust-and-js-disagree.md)).

## Context

A collection that keeps a slice lends it out mutably, as smallvec's
`SmallVec` and arrayvec's `ArrayVec` do:

```rust
impl<A: Array> AsMut<[A::Item]> for SmallVec<A> {
    fn as_mut(&mut self) -> &mut [A::Item] {
        self
    }
}
```

A type's own `AsMut` or `BorrowMut` was an error, and stopped smallvec,
arrayvec, so url and rust_decimal.

## Decision

**A type's own `impl AsMut` or `impl BorrowMut` is allowed, and called on
the type as its other methods are:** `stack.as_mut()[0] = 9` is
`stackAsMut_i32__as_mut(stack)`, the array the type keeps, written in
place.

- **Still errors:** `as_mut()` or `borrow_mut()` of a generic `T: AsMut<U>`
  or `T: BorrowMut<U>`. A dictionary of them would hand out a `&mut`, which
  for what isn't a JS object is a handle (ADR 0099); none of the measured
  crates needs it.
- **The crate's own `borrow_mut()` is the crate's,** though its trait needs
  `Borrow`: ADR 0167's refusal is for std's functions only.

## Why

- **It's exact:** the `user_as_mut` corpus case changes a struct's `Vec`
  and an array through both, by index, `sort`, `reverse`, `swap` and
  `iter_mut`, compared with native Rust.
- **It's the JS a person writes:** a method that returns the array.
