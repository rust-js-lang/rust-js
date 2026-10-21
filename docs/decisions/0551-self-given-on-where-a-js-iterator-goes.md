# 0551. A parameter given as it is, given on where a JS iterator goes, is made one

Status: Accepted. Extends [0061](0061-generic-iterators.md),
[0411](0411-an-iterator-given-as-it-is-has-its-dictionary.md) and
[0445](0445-extend-and-collect-in-generic-code.md).

Case: C ([0262](0262-when-rust-and-js-disagree.md)): what a generic
function steps of what it's given is what Rust's steps.

## Context

itertools' `next_tuple` is a default of its `Itertools`, whose `Self` is
given as it is (ADR 0411), and gives `self`, a `&mut Self`, to
`T::collect_from_iter_no_buf(self)`, a method called through `T`'s
dictionary, whose `I: IntoIterator` is given a JS iterator (ADR 0061).
`self` went as it was, a range's `{ start, end }`, which has no `next`: a
call through a dictionary resolves to no function rustc names, so the
analysis of what each function steps saw nothing given on; a reference,
`&mut Self`, wasn't taken for the parameter; an `IntoIterator` wasn't one
given a JS iterator; and the crate's own trait's call made nothing one.

## Decision

**A parameter given as it is, or a reference to one, given on where a
JS iterator goes, is made one by its dictionary, `$iterator(self,
IIterator.next, true)`, where it's given on: to a function, or a method
called through a dictionary, the crate's trait's as std's.**

- The analysis passes what a method called through a dictionary is given
  on to the method as its trait declares it, a reference as what it refers
  to; and an `IntoIterator` parameter is given a JS iterator, as an
  `Iterator` one is.
- **`given_by_its_trait` is the one answer to which of a method's
  parameters its trait's impl takes as they are**, `Self` and the trait's
  own: never made a JS iterator, nor passed on, which only the method's
  own parameters are.

## Why

`self_given_on_as_into_iter` gives a default's `&mut Self` of a range on
through a dictionary, and a generic function's `&mut I` of an array's
iterator and of a range directly, each stepped on after, against native
Rust.

## Consequences

- itertools' `next_tuple` and `collect_tuple` run.
- Mutations pass nothing on through a dictionary, a reference not as its
  parameter, an `IntoIterator` as given nothing, a trait's `Self` as a
  method's own, and leave what's given on as it is.
