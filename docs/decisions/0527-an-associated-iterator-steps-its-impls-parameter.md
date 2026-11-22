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

## Amendment: a trait method's `-> impl Iterator` is read for the type it hides (#2342)

Case: C ([0262](0262-when-rust-and-js-disagree.md)): a run that threw
now runs as Rust does.

A trait method that gives `-> impl Iterator` has an associated type of the
impl's, made by rustc: its value is the opaque type of the impl method's
result, not the type that method's body gives. So `type_of` of it was never
`I`, and the rule above never said the impl steps `I`. When the impl gave
back its own `I` held as it is (`Clone` reads it), its dictionary entry
made a JS iterator of the struct by `Iterator.from`, as it wasn't given
`I`'s `Iterator`: native Rust's `18` became "undefined is not a function",
in both builds.

**The rule reads an opaque type of the crate's for the type it hides**,
`I` of `self.0.clone()`. Then the impl takes `I`'s `Iterator`, and each
extension the method's bounds declare, and its entry steps the value by
them: `$given(givenSource_make(given, n, IClone), IIterator)`.

Since #2330, a trait's own associated iterator is its type's own value, so
this is the one path that still reaches `made_js_iterator`'s range,
crate-iterator and as-is arms. Corpus case
`trait_method_impl_iterator_stepped_by_generic_code` covers a range, the
crate's own iterator, std's `take`, and the impl's own `I` given back,
stepped by generic code's `for` and from its back by `DoubleEndedIterator`,
each run as native Rust runs it, in both builds. Mutations:
`impl-trait-method-iterator-hidden-unseen`; and
`made-item-iterator-param-as-it-is` and `dictionary-iterator-unmade`
retargeted to that case, as their earlier tests no longer reach these arms.

