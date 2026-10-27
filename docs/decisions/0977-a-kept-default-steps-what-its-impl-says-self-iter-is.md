# 0977. A kept default steps what its impl says `Self::Iter` is

Status: Accepted, under the user's goal of 2026-10-26. Extends
[0492](0492-a-kept-default-steps-its-impls-iterator.md): what a kept default
steps of its `Self` or a trait parameter, its impl's argument steps; now
also what it steps of an associated type of its `Self`.

Case ([0262](0262-when-rust-and-js-disagree.md)): results change. A range
or the crate's own iterator given to such a default threw.

## Context

```rust
trait Digits: Sized {
    type Iter: Iterator<Item = u8>;
    fn digit_iter(self) -> Self::Iter;
    fn digit_sum(self) -> u32 {
        self.digit_iter().map(|d: u8| d as u32).fold(0, |s, d| s + d)
    }
}
impl<I: Iterator<Item = u8>> Digits for I {
    type Iter = I;
    fn digit_iter(self) -> I { self }
}
```

The default steps `<Self as Digits>::Iter`, a projection, which the steps
analysis (`iterator_steps`) didn't follow into the impl, whose `Iter` is
its `I`. So `I` was "stepped by nothing" and given no dictionary:

- In the normal build the default `.reduce`d what it was given as if a
  JS iterator. `(1u8..4).digit_sum()`, `{ start, end }`, threw
  `TypeError: ... .reduce is not a function`, and so did a crate's own
  iterator.
- With std's structs alone, the `Map` it makes needed `I`'s dictionary
  and was refused, `evidence_paths` in the iterator forms' `differs`.

## Decision

What a trait's default steps of `<Self as Trait>::Assoc` is recorded with
the associated type. Each impl that keeps the default steps what it says
that type is, where that's a type parameter of the impl's, `I` of `type
Iter = I`, by the same traits. The impl's dictionary then takes `I`'s,
and the default steps it by `$given`, as any generic code steps a type
parameter given as it is.

## Consequences

`evidence_paths` leaves `differs`, which is down to one. Its snapshot's
`iDigits` takes `I`'s `Iterator`.

`test/corpus/kept_default_steps_assoc.rs` checks a range, the crate's
iterator and a `Vec`'s against native Rust. `kept-default-assoc-unstepped`
is its mutation.
