# 1001. What generic code steps from its back is stepped of its callers' types

Status: Accepted. Extends [0545](0545-a-map-stepped-from-its-back-is-made-knowing-it.md).
Fixes #388. Amended: a copy too, `Copied<I>`, and a map whose closure has a destructor (#385).

Case ([0262](0262-when-rust-and-js-disagree.md)): results change. The
program threw where Rust steps from the back.

## Context

```rust
fn back<I: DoubleEndedIterator<Item = u32>, P: FnMut(&u32) -> bool>(mut it: std::iter::Filter<I, P>) -> Option<u32> {
    it.next_back()
}
back(v.iter().copied().filter(|x| x % 2 == 1))
```

The back-stepped analysis found `Filter<I, P>` stepped from its back, of
`back`'s own type parameters. The caller makes
`Filter<Copied<Iter<u32>>, {closure}>`, which it never named, so the
filter was JS's `it.filter(p)`, with no back: `TypeError: it.back is not
a function`, where Rust gives `Some(5)`.

## Decision

The analysis keeps what each body steps from its back by its function.
What holds a type parameter is carried to each call of the function:
substituted with the call's types, it's what that caller steps, and
through what calls that caller in turn. It stops after 32 calls deep, as
a function calling itself with a type made of its own types would go on.
A caller's own type that looks like its callee's, `Filter<I, P>` of both,
is checked in the caller's set, not the crate's.

## Consequences

`test/corpus/adapters_given_to_generic_backs.rs` checks a filter and a map
given to generic code, directly and through another generic function,
against native Rust. The mutation is `generic-back-steps-uncarried`.

The probing found two of #382's survivors dead, now that a held `Zip` is
std's struct (ADR 0997):
- `iterator_steps`' rule asking an uncompiled adapter's impl what it asks
  of what it holds. Every such impl asks the same trait.
- `impl_asks`' looking through a reference.

Both go with their mutations, `adapter-impl-asks-unread` and
`adapter-impl-asks-of-references`.

## Amendment: a copy too (#385)

`fn copied_back<I: DoubleEndedIterator<Item = &u32>>(mut it: Copied<I>)`
calling `it.next_back()` was refused: "`next_back()` of a lazy or generic
`Copied`". The analysis recorded maps and filters alone, and a copy was
taken to know its back where its source does, which `I` doesn't in the
generic code. A copy is recorded as a map is, and not when it's of an
array's items, which know their back already. So the caller gives
`$iter(v)` or `$filterEnds(..)` and the generic code steps it, `$nextBack(it)`.

- `test/corpus/copied_given_to_generic_backs.rs` checks a copy given
  directly, passed on, and of a filter, against native Rust.
- `backs_given_behind_impls.js`: a copy behind an `impl`, now made knowing
  its back, asks its length as `c.len()`, where it was `c.end - c.at`.
- The mutation is `generic-copied-back-unstepped`.

## Amendment: a map whose closure has a destructor (#385)

`ends(v.iter().copied().map(move |x| x + m.0))`, `m` a value with a
destructor, given where generic code asks `DoubleEndedIterator +
ExactSizeIterator`, was refused as an `ExactSizeIterator`. Such a map is
closed to drop its closure and held in a variable, so its form, which
the length and back rules read in place, isn't seen where it's given.
And a map of an array's items was "native", stepped from its back by
that form, so the analysis didn't record it.

A map whose closure has a destructor is never native. Where generic code
asks its back, or its length alone (`lens(v.iter().map(f))`), it's
recorded: made `$mapEnds` where it's made, closed, as `$closing` passes
on its back and its length.

- `test/corpus/closing_maps_given_counted.rs` checks both, and the
  closure's drop, against native Rust.
- The mutations are `closing-map-asked-unrecorded` and
  `closing-map-length-unasked`.
- Still refused: such a map of a `fuse()`, as a `Fuse` doesn't count its
  length by its source.
