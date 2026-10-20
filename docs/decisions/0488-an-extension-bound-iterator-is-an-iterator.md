# 0488. An iterator bound by an extension alone is bound by `Iterator`

Status: Accepted. Extends [0411](0411-an-iterator-given-as-it-is-has-its-dictionary.md)
and [0487](0487-an-iterator-given-as-it-is-has-its-extensions-dictionaries.md).

Case: C ([0262](0262-when-rust-and-js-disagree.md)): each step is the one
Rust's would take, of the value Rust's code has.

## Context

A type parameter is given as it is where it's bound by `Iterator` and a
type holds it or another bound reads it (ADRs 0411, 0474). One bound by
`DoubleEndedIterator` or `ExactSizeIterator` alone, `Iterator` its
supertrait, wasn't: `struct Ends<I>` with `impl<I: DoubleEndedIterator>`
took its `inner` as a JS iterator, which its caller's value, a range or the
crate's own iterator, isn't, and threw. either's `IterEither` is bound so,
and so was refused once ADR 0487 gave it a dictionary: its `rfold` of
`inner.map(Left)`, a map of one, held in a variable, didn't know its back.

## Decision

**An iterator's extension bound is an `Iterator` bound too, as Rust
elaborates it: a type parameter bound by one is given as it is where one
bound by `Iterator` would be, and given its `Iterator` dictionary, where
it's stepped by it, before the extension's.**

- **One place makes a clause's evidence, `bounds_of`**: its own bound, and
  the `Iterator` an extension's on a type parameter given as it is
  implies, of a function's bounds and of its own alike.
- **What steps it by `Iterator` too**: an extension's method other than
  the one its dictionary holds, `nth_back` or `rfold`, runs on a JS
  iterator made of it, `$iterator(it, IIterator.next, true,
  IDoubleEndedIterator.next_back)`; `recognition::extension_entry` names
  the one each holds.
- **A std adapter of it knows what it's given**: a `map` or a `copy` of
  it is made of the JS iterator made of it, given its back or its length
  where its dictionary is given, so a map of it held in a variable, then
  stepped from its back, is `$mapEnds` of one that knows it. The analysis
  finds an adapter of it stepped from its back or counted, `map(f).rfold`,
  as stepping it so.

```js
const Ends = {
  pop(ends, IDoubleEndedIterator, IExactSizeIterator) { .. },
  shift(ends, IIterator) {
    return IIterator.next(ends.inner);
  },
};
```

## Why

- **It's Rust's meaning**: `I: DoubleEndedIterator` is an iterator; the
  `extension_bound_iterators` corpus case steps one a type holds from
  either end, counts it, and takes `nth_back` of a cloned one, of a range,
  a vector's and the crate's own iterator, and `extension_map_rfold`
  folds a held map of one from its back, with native Rust.
- **One way**: an extension bound is read as rustc reads it, so the two
  spellings, `I: Iterator + DoubleEndedIterator` and `I:
  DoubleEndedIterator`, are lowered alike.

## Consequences

- **What compiles grows**: either's `IterEither` and `Either` as
  double-ended and exact-size iterators, and any type holding an iterator
  bound by an extension alone.
