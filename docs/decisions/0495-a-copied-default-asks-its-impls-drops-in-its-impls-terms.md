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

## Amendment: a default's own parameters are its own, numbered after the impl's (#802, #804)

A copied default's own type parameters, `U` of `fn counted_with<U>`, were in
the impl's terms numbered as in the trait's, after `Self`, which the impl's
own parameter of that number is too: `V` of `impl<I, V> Counting for
Paired<I, V>`. So a `U` was given `V`'s drop, the impl's argument of its
index (#804). And none of its own drops, which its caller gives after its
dictionaries as the trait declares them (ADR 0163), was taken: what it was
given of a `U` was never dropped (#802). Case C of ADR 0262.

- **One mapping puts a default in its impl's terms, `traits::default_args`**:
  its trait's parameters the impl's arguments for them, its own each a
  parameter numbered after the impl's. Its lowering, its slots
  (`slot_bounds`, ADR 0478) and the analysis of what it asks (`type_facts`,
  #737) all ask it; `test/architecture-traits.test.ts` holds it.
- **One environment, `traits::default_typing_env`, where its body is
  lowered**: the impl's bounds, its own in its trait's terms, and those of
  its own parameters alone, and its `Self: Sized`, in the impl's. It
  replaces `impl_typing_env`, a second environment beside the body's, and
  the analysis's own.
- **Its own drops are its caller's**, as an impl's generic method's entry
  takes them (`own_drops_taken`), under its own parameters' numbers in
  both terms, and the impl's while it's lowered (`with_impl_drops` asks
  them there, `impl_owned`); those after the last it uses left out.

```js
counted_with(self, extra, dropU, dropUItem, dropUIntoIter) { ... }
```

Held by `test/corpus/default_own_param_drops.rs` and
`default_own_param_numbered_as_the_impls.rs`, with native Rust. A default
whose `Self`, of an impl that isn't a blanket one, is stepped as a JS
iterator is #803's.
