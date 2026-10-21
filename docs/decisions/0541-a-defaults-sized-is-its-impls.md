# 0541. A copied default's `Self: Sized` is its impl's

Status: Accepted. Extends [0107](0107-const-generics.md) and
[0536](0536-a-blanket-unsized-impl-keeps-its-defaults.md).

Case: D ([0262](0262-when-rust-and-js-disagree.md)) closed: what was
rust-js's own error compiles as Rust's does.

## Context

itertools' `position_minmax_by_key`, a default of `Itertools`, calls its
own trait on an adapter of `Self`, `self.enumerate().minmax_by_key(..)`,
where `Self: Sized`. Copied into the blanket impl over `?Sized` types,
`impl<T: Iterator + ?Sized> Itertools for T` (ADR 0536), its types are the
impl's, `Enumerate<T>`, whose dictionary is the impl's again. rustc chose
no impl for it: `Enumerate<T>` needs `T: Sized`, and the typing
environment had the default's `Self: Sized`, in its trait's terms, not the
impl's. It was refused, "implementation evidence for `<Enumerate<T> as
Itertools>`".

## Decision

**A copied default's typing environment has its own bounds, in its
trait's terms, which its body names, and its `Sized` ones in its impl's
too**, `T: Sized`. No other in the impl's terms: rustc chooses a `where`
clause over the impl it meets, which has the dictionary.

## Why

`default_on_an_adapter_of_self` finds the position of the greatest item
by a default calling its trait on `self.enumerate().map(..)`, of a `Vec`'s
iterator, `chars()` and an empty one, against native Rust.

## Consequences

- itertools' `position_minmax_by_key` compiles.
- A mutation leaves the `Sized` bound in the trait's terms.
