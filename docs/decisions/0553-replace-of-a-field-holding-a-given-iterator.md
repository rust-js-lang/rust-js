# 0553. `mem::replace` of a field holding an iterator given as it is

Status: Accepted. Extends [0147](0147-replacing-through-mut.md) and
[0411](0411-an-iterator-given-as-it-is-has-its-dictionary.md).

Case: C ([0262](0262-when-rust-and-js-disagree.md)): what `mem::replace`
gives back and leaves behind is Rust's.

## Context

`mem::replace(&mut self.orig, self.iter.clone())`, where `orig` holds a
type parameter given as it is, an `I: Iterator + Clone`, was refused:
"this `&mut` given to `mem::swap` and the like". `mem::replace` wrote a
place only where its `&mut` was a variable's, or changed an object in
place, and an iterator given as it is is neither by type, though `*r = v`
of one is written in place, `$assign` (ADR 0411). In a closure, which
captures `self.orig` itself, there's no field to write.

## Decision

**`mem::replace`, `mem::take` and `mem::swap` of a `&mut` borrowing a
field, `&mut self.orig`, write the field, as of a variable; of another,
what's written in place is changed in place, `$take(captured, value)`, as
`*r = v` of it is `$assign`.**

- **`written_in_place` is the one answer to what a `&mut` to a type
  changes in place to write it**: an object, or an iterator given as it
  is, never a cell. `*r = v` and `mem::replace` both ask it.
- `borrows_field` asks THIR, as `borrows_variable` does, whether the
  call's `&mut` borrows a field.

## Why

`replace_held_given_iterator` replaces a field holding a range and a
`Vec`'s iterator, given as they are, in a method and in a closure that
captures the field, against native Rust.

## Consequences

- Any field `mem::swap`, `mem::replace` or `mem::take` borrows is written
  as a place, `const t = pair.left; pair.left = pair.right; pair.right =
  t;`, not changed in place, `$exchange(pair.left, pair.right)`: nothing
  else names it while the call has it, as borrowck says, so no one sees
  the difference, and it reads as Rust's. `swap_replace`'s snapshot shows
  it.
- Mutations take the field for no place, refuse what's written in place
  but isn't an object, and write an iterator given as it is otherwise.
