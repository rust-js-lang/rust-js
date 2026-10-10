# 0340. A slice of `MaybeUninit`s, and a `const` block of no JS value

Status: Accepted. Extends [0332](0332-maybe-uninit.md),
[0127](0127-const-blocks-and-let-guards.md) and
[0096](0096-statics.md); counted by [0314](0314-std-data-structures.md).

Case: A ([0262](0262-when-rust-and-js-disagree.md)).

## Context

A slice of `MaybeUninit`s' `write_copy_of_slice`, `write_clone_of_slice`,
`assume_init_ref`, `assume_init_mut` and `assume_init_drop` were errors.
So was the array of slots a type that isn't `Copy` needs,
`[const { MaybeUninit::<T>::uninit() }; N]`: a `const` block was only
the value rustc computes as a value tree, which a union such as
`MaybeUninit`, or a `String`, has none of.

## Decision

**A slice of `MaybeUninit`s is the array of what they hold (ADR 0332);
its methods are a slice's. A `const` block whose value rustc can't say
as JS's is lowered as code, a constant of its module, as a named one is
(ADR 0096).**

```rust
let mut slots = [const { MaybeUninit::<Noisy>::uninit() }; 2];
let cloned = slots.write_clone_of_slice(&src);
unsafe { slots.assume_init_drop() };
```

```js
const mainConst = undefined;
...
const slots = new Array(2).fill(mainConst);
const cloned = $writeCloneOfSlice(slots, src, (value) => noisyClone_clone(value));
for (const item of slots) {
  noisyDrop_drop(item);
}
```

| Rust | JS |
|---|---|
| `slots.write_copy_of_slice(src)` | `$copyFromSlice(slots, src)`, then `slots` |
| `slots.write_clone_of_slice(src)` | `$writeCloneOfSlice(slots, src, clone)`: a clone of each, panicking as its `assert_eq!` does |
| `assume_init_ref()`, `assume_init_mut()` | the slots themselves |
| `assume_init_drop()` | each item dropped |
| `const { MaybeUninit::uninit() }`, `const { String::new() }` | a constant of its module, `mainConst`, named by its function |
| `[const { .. }; N]` of a value that isn't `Copy` | `N` uses of the constant, each its own where something changes one |

- **A `const` block is decided where it's used**, of that use's types:
  one rustc's value says as JS's is that value in place, as before.
- **One in generic code stays refused**: its value is its caller's.
- **A constant `MaybeUninit` is one JS value of every use**, as what it
  holds would be, and a `String` is its JS string; a `Vec` is not, as
  each use of a constant one is made afresh and changed in place, and
  stays refused, loud (`const_block_vec`).

## Why

- **It's the same program**: each slot written, cloned, read and dropped
  as Rust's is, and each use of a constant is its value.
- **It's tested**: the `slice_uninit` corpus case runs, against native
  Rust, `write_copy_of_slice` changed through what it gives,
  `assume_init_ref` and `assume_init_mut`, and `write_clone_of_slice` and
  `assume_init_drop` of a type that prints its clones and drops;
  `slice_uninit_lengths` panics as `assert_eq!` does; `const_block_values`
  uses `const` blocks of a `String` and a `MaybeUninit`;
  `const_blocks_and_let_guards` keeps its values in place. Mutations drop
  the coded constants or code every one, leave one unnamed, typed by its
  placeholder or asked its visibility, refuse the repeat, unshare
  `MaybeUninit` and `String`, skip each write, clone and drop, and the
  `assert_eq!`'s sides.
- `docs/std-coverage.txt`: `slice` 120 of 133.
