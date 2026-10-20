# 0495. A copied default asks its impl's drops in its impl's terms

Status: Accepted. Amends [0492](0492-a-kept-default-steps-its-impls-iterator.md),
extends [0098](0098-destructors.md) and [0049](0049-traits-and-generics.md).

Case: C ([0262](0262-when-rust-and-js-disagree.md)): each value dropped by
its own drop, as Rust drops it.

## Context

A copied default is given its trait's type parameters' drops, by their
indexes, `dropSelf` for `Self`, in place of the function's (ADR 0098). What
its body calls in its impl's terms, the impl's own method or a dictionary
of the impl, is given drops for the impl's type parameters, whose indexes
the trait's share: of `impl<I: Iterator> Trait for Holder<I>`, the impl's
`I` is index 0, as the trait's `Self` is. So `I`'s drop was `dropSelf`, a
drop of a `Holder`, given to what drops an `I`; a nested default's by-value
`self` was then dropped as the inside of what it is, and threw.

## Decision

**What a copied default makes in its impl's terms is asked of the impl's
drops: the arguments of the impl's own method its body calls, and the
dictionary, and its method's own evidence, of a call through the impl's
trait.** One place, `with_impl_drops`, puts the function's drops back in
the default's place while it makes them, `outer` keeping each function's
drops a copied default is lowered in.

```js
skip_one: (self) => {
  const value = holderIterator_next(self, IIterator, dropI, dropIItem);
```

## Why

- **It's Rust's**: `copied_default_impl_drops` calls an impl's `next`,
  which drops the iterator it replaces, and defaults through the impl's
  dictionary, nested and by value, of a type with a destructor, directly
  and through a `dyn`, with native Rust.

## Consequences

- **A default's body's own types are still asked in its trait's terms,
  `dropSelf`**; only what it makes in the impl's is the impl's.
