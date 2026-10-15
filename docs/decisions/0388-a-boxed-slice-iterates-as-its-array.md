# 0388. A boxed slice iterates as its array

Status: Accepted. Extends [0023](0023-strings-references-shared-state.md)
and [0061](0061-generic-iterators.md).

Case: N ([0262](0262-when-rust-and-js-disagree.md)): a `Box<[T]>` is the
JS array a `Vec<T>` is, and iterates the same.

## Context

A `Box<[T]>` is its value, the array (ADR 0023), and std has iterated one
by value since Rust 1.80, its `into_iter()` a `vec::IntoIter`. rust-js knew
the `into_iter()` of a `Box<[T; N]>` and refused a `Box<[T]>`'s:

```rust
let numbers: Box<[i32]> = vec![1, 2, 3].into_boxed_slice();
let doubled: Vec<i32> = numbers.into_iter().map(|n| n * 2).collect();
```

So was rustc's `iterators/into-iter-on-boxed-slices-lint.rs`.

## Decision

**A `Box<[T]>`'s `into_iter()`, by value, through a `&` or a `&mut`, is
its array's, as a `Box<[T; N]>`'s is: the one rule of a boxed array covers
a boxed slice.**

```js
const doubled = numbers.map((n) => Math.imul(n, 2));
for (const item of items) {
  item.count = (item.count + 1) >>> 0;
}
```

## Why

- **It's how JS iterates an array**, and what a `Vec`'s gives.
- **It's tested**: the corpus's `boxed_slice_iter.rs` iterates one by
  value, through a `&` and through a `&mut` that changes its items; the
  rustc test passes.
