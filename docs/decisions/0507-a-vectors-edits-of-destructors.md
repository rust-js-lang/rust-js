# 0507. A vector's edits of values with a destructor

Status: Accepted. Extends [0098](0098-destructors.md) and
[0324](0324-slice-methods.md).

Case: C ([0262](0262-when-rust-and-js-disagree.md)): what's dropped, and
what's a clone, would change, so Rust's is kept, paid for in output.

## Context

A port of a `Vec` of values with a destructor met refusals for std's own
edits: `fill`, `resize`, `clone_from_slice`, `dedup`, `extend_from_within`,
`first_mut`, `last_mut`, `split_at_mut`, `split_first_mut`, `chunks_mut`
and the rest of a slice's lending views. Finding them found three more
gaps, none of them about destructors:

- `dedup()` of anything `===` doesn't compare, a struct's, was refused.
- `extend_from_within` pushed the same objects again, not clones: a change
  to one changed both.
- `fill` cloned each, where std's calls the type's own `clone_from`.

## Decision

**Each drops what std's drops, where std's drops it, and clones what std's
clones, by the same `clone_from`:**

- **`fill(x)`**: each but the last written over by its type's `clone_from`
  of `x`, or a clone, the one replaced dropped; the last is `x`, the one it
  replaces dropped; of no items, `x` dropped:
  `$fill(v, x, clone, cloneFrom, drop)`.
- **`clone_from_slice(src)`**: the same, of each item of `src`:
  `$cloneFromSlice(v, src, clone, cloneFrom, drop)`.
- **`resize(n, x)`**: shortened as `truncate` drops what's past it, then `x`
  dropped; or lengthened with clones and `x` last: `$resize(v, n, x,
  clone, drop)`.
- **`dedup()`** is std's `dedup_by(|a, b| a == b)`: `$dedupBy(v, eq, drop)`;
  of what `===` compares and drops nothing, `$dedup(v)`, as before.
- **`extend_from_within(range)`** pushes clones, where a clone is more than
  the value: `v.push(...$copyRange(v, a).map(clone))`.
- **`first_mut`, `last_mut` and the slice ops that lend items or views of
  them** drop nothing. `SliceOp::lends` is the one answer to which those
  are.

`clone_over`, the slice's, is the one answer to how an item is written over
by a clone of another, the type's own `clone_from`, a type parameter's
dictionary's, or a clone; `fill` and `clone_from_slice` both ask it. And
`Expr::optional_args` is the one way a helper's trailing arguments are
given, `undefined` for an absent one before a given one: the five copies of
that loop, the dictionary drops' and a map's, go through it.

## Why

- **It's the same program**: `slice_edit_drops` runs each edit of values
  with a destructor, `fill` of a type with its own `clone_from` and of a
  unit enum with one, and `dedup` of a struct, against native Rust.

## Consequences

- `fill`'s and `clone_from_slice`'s helpers take the same arguments in the
  same order, `clone, cloneFrom, drop`.
- Mutations drop none of what each replaces, cuts off or is given, compare
  structs by `===`, push the same objects, refuse each edit, and refuse a
  type's own `clone_from`.
