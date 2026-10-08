# 0176. A generic impl's constant of its parameters is its dictionary's getter

Status: Accepted. Extends [0106](0106-generic-traits.md) and
[0031](0031-consts.md).

Case: C, D ([0262](0262-when-rust-and-js-disagree.md)).

## Context

ADR 0106 left out a generic impl's constant made of its parameters':

```rust
impl<T: ConstZero> ConstZero for Wrapping<T> {
    const ZERO: Self = Wrapping(T::ZERO);
}
```

rustc computes a constant's value for the program to use, but this one has
no one value: it's `T`'s, for each `T`. num-traits has two, which stopped
it, and chrono and rust_decimal with it.

## Decision

**A generic impl's constant rustc can't compute is a getter of the impl's
dictionary, its initializer lowered there, where the impl's evidence is:**

```js
get ZERO() {
  return [TConstZero.ZERO];
}
```

- **Read each time, as a constant is:** each use of a Rust constant is a
  value of its own, so a getter is, where a value changed in place could
  otherwise be shared (ADR 0031).
- **Only where rustc can't compute it:** one of no parameter, or of known
  types, is the value rustc computed, as before.
- **Still an error:** a trait's default constant of `Self` the generic impl
  keeps, which would need its `Self` read as the impl's type.

## Why

- **It's exact:** the `generic_impl_consts` corpus case reads a
  `Wrapping(T::ZERO)` and an `A::BITS + B::BITS`, nested and through generic
  code, with native Rust.
- **It's the JS a person writes:** the value, worked out from the
  parameters' own, where it's asked for.
