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

## Amendment: a copied default's body is in its impl's terms (#803)

A default copied into an impl that isn't a blanket one, `impl<I: Iterator>
Keeping for Wrapper<I>`, read its `Self` half in its trait's terms, a type
parameter's JS iterator (`Iterator.from(self)`, `$intoIter(self)`), and
half in the impl's, the struct's own `next`, so `self.map(Some).collect()`
of a `Wrapper(Countdown(20))` gave one item, not two, and a `Paired`'s
`self.count()` counted its fields. Case C of ADR 0262.

- **Its body is lowered from its MIR in the impl's terms**,
  `Mir::in_terms_of` of `default_args`, normalized where
  `default_typing_env` knows it, and each closure in it too
  (`copied_default_args`). Nothing in it is in its trait's terms any more:
  `in_impl_terms`, the drops' trait-terms mapping, `with_impl_drops`
  (this ADR's decision) and `through_entry` (ADR 0454, #690) are gone.
- **Its environment is the impl's and its own where clause in the impl's
  terms** (`default_typing_env`), normalized there, `T::Item:
  IntoIterator` of its `Self::Item: IntoIterator`; not its trait's `Self:
  Trait`, which would be chosen over the impl that meets it. Its const
  parameters are the impl's and its own, numbered after the impl's: a
  default's `N` numbered as the impl's `K`, refused before (ADR 0135), now
  compiles.
- **It drops as the impl does, with the impl's drops**
  (`impl_drops_for_default`), its own type parameters' drops beside them
  (#802, #804), the impl's associated types' under indices past every
  parameter's.
- **Its entry takes what its caller through the dictionary gives**, in the
  trait's forms, made the impl's at the top of its body, as an impl
  method's entry makes them (`entry_forms`): `const c = self.value;` of a
  `&mut self` generic code gives in a box. One that gives back a `&mut`
  its impl holds otherwise is refused.
- The box its entry takes of a `self` is named `self`, but `boxed` where
  the `self` it holds keeps that name, one of no struct
  (`analysis::naming::self_box_name`): a box is never named as what it
  holds. Its `&mut self` given on to generic code is boxed again and copied
  back, as an impl's own method's is (ADR 0454), where it was the box it was
  given (#809, P3; `docs/readability.txt` blessed for it).
- Its `self` is named after its type, as an impl's own method's is
  (`(meters) => ..`), and what generic code needed, `$someAt`, `$held`,
  `dropSelf`, goes where the types are known.

```js
twice(self) {
  const c = self.value;
  cCounter_bump(c);
  cCounter_bump(c);
},
```

Held by `test/corpus/default_of_a_non_blanket_impl_steps_its_self.rs`, with
native Rust, its `for _ in self` too, as generic code's `for` over a
`Wrapper<I>` is since #806; about 40 snapshots move (the user's approval,
through m6-react).
