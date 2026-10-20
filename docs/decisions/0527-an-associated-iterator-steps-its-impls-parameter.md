# 0527. An associated iterator steps its impl's parameter

Status: Accepted. Extends [0411](0411-an-iterator-given-as-it-is-has-its-dictionary.md).

Case: D ([0262](0262-when-rust-and-js-disagree.md)) closed: what was
rust-js's own error, an analysis that missed a step its lowering asked
for, compiles as Rust's does.

## Context

A type parameter given as it is is given its `Iterator` dictionary only
where something steps it (ADR 0411): `stepped_params` finds each function,
and each impl one of whose items does. An impl whose associated type is an
iterator, `IntoIterator for &RcIter<I>`'s `IntoIter = RcIter<I>`, or
itertools' `HasCombination`'s `Combination = Tuple2Combination<Fuse<I>>`,
steps nothing in a body, but its dictionary gives that type's `Iterator`
dictionary, made of the impl's evidence, which steps `I`. A library, which
makes each impl's dictionary, refused it: "stepping `I`, which rust-js
found nothing here steps". itertools had twelve, hidden behind `tee`'s
refusal until [0523](0523-libraries-share-one-counted-rc.md).

## Decision

**An impl's associated type bound by `Iterator` steps what that type's
`next` steps.** Of each, `stepped_params` resolves the type's `next`, the
crate's own: a parameter passed on as it is is passed to it, as a
function's argument is; one held in a std adapter, `Fuse<I>`, is stepped
by the impl.

## Why

`associated_iterator_steps` is an `IntoIterator` of a reference whose
`IntoIter` holds its parameter, and a trait's `type Out: Iterator +
From<I>` of a `Fuse<I>`, used by generic code, built as a program and as a
library, against native Rust.

## Consequences

- itertools' twelve are gone; its next refusals are the iterators draft's
  and a few of their own.
- Mutations skip associated types, pass nothing on, or step nothing held
  in an adapter.
