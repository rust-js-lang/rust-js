# 0483. `BorrowMut` is given a dictionary

Status: Accepted. Extends [0167](0167-borrow.md).

Case: A ([0262](0262-when-rust-and-js-disagree.md)): the same value.

## Context

`key.borrow()` of a `K: Borrow<Q>` is its dictionary's `borrow` (ADR
0167). `BorrowMut`, `Borrow`'s counterpart, had no dictionary: rust-js
lowered the crate's impls of it, called on the type itself, but gave a
bound by it nothing. A trait whose supertrait it is, itertools'
`PoolIndex<T>: BorrowMut<[usize]>`, then had no way to its `Borrow`: its
own default, `self.borrow().len()`, lowered as the trait's for a library's
other impls, was refused.

## Decision

**`BorrowMut` is `operational`**: a bound by it is given its dictionary,
`{ borrow_mut }` and its `Borrow` supertrait's entry, and a trait's
supertrait chain reaches `Borrow` through it. std's, of what borrows as
itself as an object, a `Vec`'s or an array's slice, is the same array; of
text, a value JS can't lend in place, there's none. A crate's impl's is
written only where generic code reads it (`read_only_generically`).

```js
$vecPoolIndex = { len: (self) => ({ borrow: (value) => value }).borrow(self).length, ... };
```

## Why

- **It's std's**: `borrow_mut_supertrait` reads and writes through a
  trait's `BorrowMut<[usize]>` supertrait, of a `Vec` and an array, through
  generic code and as a library, with native Rust.

## Consequences

- **What compiles grows**: `borrow_mut()` through a bound, and `borrow()`
  through a `BorrowMut` supertrait.
