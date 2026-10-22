# 0623. A copy std makes of a `Copy` value is a copy

Status: Accepted.

Case ([0262](0262-when-rust-and-js-disagree.md)): results would change,
so Rust's.

## Context

A `Copy` struct, array or tuple is a JS object, and a copy of one,
`let b = a`, is `{ ...a }` where something changes one in place
(`needs_clone`). `cloned()` and `to_vec()` asked so; what std copies
otherwise didn't (#98): `copied()` was the iterator it copies, so
`v.iter().copied().collect()`, `.next()`, `.find(..)` and the rest gave
the slice's own objects; so did `extend(&v)` and `partition` into a
collection of `T`, which `Extend<&T>` copies into, `copy_from_slice`,
`repeat`, `concat`, and an array made of a slice, `v[..].try_into()`.
Changing a copy changed the original.

## Decision

**What std copies of a `Copy` item is a copy wherever `needs_clone` says
one needs one:**

- `copied()` is its source only where its item needs no clone, as
  `cloned()`'s is; else each item mapped through `clone_value`.
- `copies_of(items, ty)` maps items through their clone where they need
  one; `copied_into(items, from, into)` does where `from` is `&T` and the
  collection holds `T`, as `extend` and `partition` put them.
  `concat`'s join is `copies_of` its items.
- `$copyFromSlice` and `$repeatItems` take a copy function, given where
  the item needs one.
- An array made of a slice is `clone_items` of it.

Where nothing changes a `T` in place, nothing changes: `needs_clone` is
false, and the JS is as it was.

## Consequences

- A generic `T: Copy`'s `copied()`, std_traits' `largest`, copies each
  item through `TCopy.copy`, though only the one it gives back needs it:
  a copy of the result alone is a later refinement.
- A slice iterator of tuples a program changes in place, copied and given
  back, `copied_given_back`, is `slice.values().map(..)` where it was
  `$iter(slice)`.

## What holds it

- `test/corpus/copied_structs_apart.rs`: each of the ways above, each
  copy changed and the original printed.
- Mutations: `copied-items-shared`, `copies-of-shared`,
  `slice-to-array-items-shared`, `partition-copies-shared`,
  `copy-from-slice-shared`, `repeat-items-shared`.
