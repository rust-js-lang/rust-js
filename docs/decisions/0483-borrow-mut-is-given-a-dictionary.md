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
`{ borrow_mut }` and its `Borrow` supertrait's entry, which it holds as
`DerefMut`'s holds its `Deref` (`holds_supertraits`), and a trait's
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

## Amendment: a value's `borrow_mut()` as itself is the `&mut` it's given (2026-11-12, #1487)

`m.borrow_mut()` giving a `&mut Vec<i32>` of a `Vec<i32>` is std's
`impl<T> BorrowMut<T> for T`. No recogniser took it of a type known where
it's called, only `BorrowMut<[T]>` and `BorrowMut<str>` (#807), so it went
by the trait call generic code makes and read generic code's box of the
value, `m.value`: a `TypeError` of a `Vec`, a struct, a tuple or a map, a
refusal of a number.

It's the `&mut` it's given, `Std::Same` (`views_itself`): the object itself,
`const s = m;`, or of a number or text the place, `n.borrow_mut()` of an
`i32` a `&mut` to `n`. Rust's behaviour is preserved (ADR 0262's "results
change"), and the JS is what a person writes. A type parameter's stays its
dictionary's. And what no recogniser takes of it no longer reaches that
dictionary (ADR 1128, amended).

`test/corpus/borrow_mut_of_a_value_as_itself.rs` (a `Vec`, the crate's
struct, a tuple, a `HashMap`, a struct's fields, an `i32`, a `String`, an
`Option`, an enum, through a `&mut` parameter and written out) runs against
native Rust in both builds. Mutation
`borrow-mut-of-a-value-as-itself-unrecognised`
(`lower/recognition/wrappers.ts`).
