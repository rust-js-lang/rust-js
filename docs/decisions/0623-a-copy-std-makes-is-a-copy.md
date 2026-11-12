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

## Amendment: a copy std makes of a `Copy` value is its bits, not its `Clone`

Case ([0262](0262-when-rust-and-js-disagree.md)): results would change,
so Rust's.

The decision above made each of those copies through `clone_value`, a
type's `Clone`: right where the `Clone` is derived or std's, which copy
the bits, but a `Copy` type may write its `Clone` by hand, and std calls
it only where its own code calls `clone`. The metamorphic net's `loud`
rewrite of `generic_copied_cloned`, whose `Clone` of a `Copy` struct
prints, found `ps.iter().copied()` printing a clone of each item, which
std's `Copied` never makes. Comparing the siblings against native Rust
found more: `copy_from_slice` called it; `partition` of `&T`s into `T`s
called it before its predicate, which then saw what the `clone` made and
put every item on the wrong side; an array made of a slice called it;
`repeat` called it for every copy, where std clones its first copy and
copies that one's bits after; and the other way round, a clone of a
`Copy` tuple, array or `Option`, an `Option`'s `cloned()` and a derived
`Clone` of a generic type copied the bits where std's clone calls each
part's `clone`.

**Where std copies a `Copy` value's bits, rust-js copies them; where it
calls `Clone`, rust-js calls it:**

- `bits_copy_fn(item)` (copies.rs) is the one function of a bits copy,
  `copy_if_needed`'s, never a `Clone`: `copied()`, an `Option`'s
  `copied()`, `copy_from_slice`, `partition`'s push of a `&T` (std's
  `extend_one`, `push(*item)`) and an array made of a slice ask it.
- `repeat`'s first copy is each item's clone and the rest its bits:
  `$repeatItems(v, n, size, copy, clone = copy)`, its clone given only
  where it isn't its copy, a hand-written `clone`'s.
- `clone_value` copies a `Copy` value's bits only where its clone runs no
  code (`copy_clone_runs_code`, clones.rs): not a type with a
  hand-written `Clone`, nor a tuple, an array, an `Option` or another
  std type, or a derived `Clone` of a generic type, holding one, whose
  clones call each part's. What std says is its bits is asked of rustc,
  never listed here: a type that implements std's `TrivialClone` (lang
  item `trivial_clone`), which `#[derive(Clone, Copy)]`, a reference and
  an array of such do in this toolchain's std, copies its bits, as std's
  own specializations on it do, so a toolchain whose std moves that line
  moves rust-js with it. A toolchain with no such lang item has none of
  these, and the walk of the parts alone decides.
- `to_vec()`, `extend_from_slice`, `extend(&v)`, `concat` and `cloned()`
  call `Clone`, as std's do, and were right.

Generic code bounded by `T: Copy` alone clones through its `Copy`
dictionary's supertrait `Clone`, which a caller gives where the type's
clone runs code ([0289](0289-copy-bounds.md), amended, #1451).

What holds it: `test/corpus/copied_of_a_copy_with_its_own_clone.rs`,
each of the above against native Rust in both builds, each copy changed
apart after; mutations `copied-calls-its-clone`, `copy-from-slice-clones`,
`bits-copy-shares`, `partition-copies-by-clone`, `array-of-a-slice-cloned`,
`repeat-clone-dropped`, `repeat-clones-each-copy`, `option-copied-is-cloned`,
`option-cloned-is-copied`, `copy-clone-runs-no-code` and
`copy-clone-derive-not-bits`.
