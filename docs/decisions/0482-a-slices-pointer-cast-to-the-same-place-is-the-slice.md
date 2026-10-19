# 0482. A slice's pointer cast to what's at the same place is the slice

Status: Accepted. Extends [0407](0407-a-raw-pointer-is-a-handle-on-its-place.md)
and [0332](0332-maybe-uninit.md).

Case: A ([0262](0262-when-rust-and-js-disagree.md)): the same items, at the
same place.

## Context

A raw pointer to a slice is the slice itself (ADR 0407), and a
`MaybeUninit<T>` is its `T`'s value (ADR 0332). A cast of one to the other,
`slice as *mut [MaybeUninit<T>] as *mut [T]`, itertools'
`slice_assume_init_mut`, was taken for a number's cast and refused: a cast
was a pointer's only of a handle on a place.

## Decision

- **A cast of or to any raw pointer is `pointer_cast`'s**, a slice's too.
- **A slice's pointer cast to one of what's at the same place**, `*mut
  [MaybeUninit<T>]` as `*mut [T]`, is the slice itself.

```js
function slice_assume_init_mut(slice) {
  return slice;
}
```

## Why

- **It's the same items**: `uninit_slice_cast` writes and shows numbers and
  text through the cast, and drops them in place, with native Rust.

## Consequences

- **What compiles grows**: a slice's pointer cast between `MaybeUninit<T>`
  and `T`.
