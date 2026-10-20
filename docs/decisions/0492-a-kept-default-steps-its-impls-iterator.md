# 0492. A kept default steps its impl's iterator as the impl does

Status: Accepted. Extends [0411](0411-an-iterator-given-as-it-is-has-its-dictionary.md)
and [0049](0049-traits-and-generics.md).

Case: C ([0262](0262-when-rust-and-js-disagree.md)): each step is the one
Rust's would take, of the value Rust's code has.

## Context

A blanket impl's method, `impl<T: Iterator> Trait for T`, steps its `T`
by its `Iterator` dictionary, as its caller gives its own value (ADR
0411's amendment). A default of the trait the impl keeps, copied into it
(ADR 0049), didn't: the analysis that gives the dictionary saw only the
impl's own items, so the default stepped its `Self` as a JS iterator,
`$nextSome(self)`, which a range or the crate's own iterator isn't, and
threw. Of an impl for the crate's iterator, `impl<I: Iterator> Trait for
Holder<I>`, `self.next()` in the default was read as any iterator's, and
its item's drop was looked for in the supertrait's dictionary,
`.Iterator().$dropItem`, which std's has none of.

## Decision

**A default an impl keeps steps the impl's argument for its `Self` as
the impl's own method would: a blanket impl's `T` by its `Iterator`
dictionary, the crate's iterator by its own `next`.**

- **The analysis follows a kept default**: what it steps of its `Self`,
  or a trait's parameter, it steps of the impl's argument, a type
  parameter of the impl's, as a function steps one passed on; of the
  crate's iterator holding them, what its `next` steps, where the default
  steps it. A trait's own parameter is never made a JS iterator by a
  caller, so none is given a dictionary for being one.
- **A call is the crate's where it resolves to the crate's in the impl's
  terms**, `resolves_to_crate`: the one answer of what's std's, of `self.next()`
  in a copied default too, so it's neither a JS iterator's step nor
  std's.
- **The dictionary it's given is asked in the impl's terms**,
  `iterator_evidence`.
- **A std trait's associated type is never looked for in a dictionary**,
  `item_drop_of`; its slot is found in the impl's terms, normalized in the
  impl's typing environment, `<Holder<I> as Iterator>::Item` as `I::Item`.
  (Amended: one whose bound is given a dictionary is its entry's, ADR
  0497; an iterator's slot is found first.)

```js
function tSkipping(TIterator) {
  ...
    skip_one: (self) => {
      TIterator.next(self);
    },
```

## Why

- **It's Rust's**: `copied_iterator_defaults` keeps defaults that step,
  sum, map, loop over and lend `Self`, of a range, a vector's and the
  crate's own iterators, items with a destructor among them, and of an
  impl for the crate's iterator holding another, with native Rust.
- **One way**: a kept default and the impl's own method step its iterator
  alike.

## Consequences

- **A blanket impl's defaults take its `Iterator` dictionary**,
  `$iterator(self, TIterator.next, true)` where a JS iterator of it is
  asked, as its own methods do.
- **A copied default's drops of the impl's own type parameters** are yet
  confused with its trait's, by index, where it calls the impl's own
  functions: a gap of its own. (Amended: they're the impl's, ADR 0495.)

## Amendment: one place asks whether a call is the crate's

`resolves_to_crate` keeps every call its impl writes itself from std's
iterator methods, so the iterator lowering's own checks of the same
question, `own_len` and `user_impl_defines`, never held: CI's mutations
of them survived once `mir/iter.rs` changed. Both are removed, and the
crate's iterator's `next` with them; `resolves_to_crate` is the one
answer.

And the iterator lowering asked again whether a method is `Iterator`'s,
`is_std_def(trait_id, Iterator)`, after `iterator_method` had answered,
so `iterator_method`'s own answer could never be seen. It now goes on only
where `iterator_method` names one of std's iterator methods: `Ord::max`
of numbers, which shares `Iterator::max`'s name, holds it.
