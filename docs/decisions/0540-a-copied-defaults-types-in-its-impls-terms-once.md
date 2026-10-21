# 0540. A copied default's types are put in its impl's terms once

Status: Accepted. Extends [0421](0421-std-associated-type-drops-are-given.md) and [0495](0495-a-copied-default-asks-its-impls-drops-in-its-impls-terms.md).

Case: N ([0262](0262-when-rust-and-js-disagree.md)): rust-js crashed;
what's dropped is as it was.

## Context

A trait's default copied into an impl names its types in its trait's
terms, `Self::Item`, which the drops it's given name in the impl's,
`I::Item` (ADR 0421): a drop is looked up as it's named, then put in the
impl's terms. One named in the impl's terms already was put in them again.
Where the impl's parameters are numbered as its trait's, `impl<I> Grab for
I`, that changed nothing; where they aren't, `impl<'a, I> CollectSlice<'a>
for I`, `I` is the impl's second parameter and the trait's second is `'a`,
and rustc panicked: "expected type for `I/#1` but found Lifetime". rustc's
`issue-71805`, a default with a `const N` of its own, crashed so once ADR
0536 compiled its impl, over `?Sized` types; over `Sized` ones it always
had.

## Decision

**A type is put in a copied default's impl's terms only where it's in its
trait's: where each parameter it names is the default's own.** The drop
query is given them, the default's identity, with the impl's arguments.

## Why

`copied_default_own_params`, `issue-71805`'s program, collects an array by
its default, its impl's `'a` first, against native Rust; without this,
rust-js crashed. rustc's `issue-71805` passes, off the known failures.

## Consequences

A mutation puts it in the impl's terms twice.
