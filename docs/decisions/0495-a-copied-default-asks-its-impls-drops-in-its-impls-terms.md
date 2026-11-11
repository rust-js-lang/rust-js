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

## Amendment: a collect in a copied default asks its types as they are (2026-11-06, #799)

Since #803 a copied default's MIR is the impl's, normalized where
`default_typing_env` knows it, so what std collects in place in, of its
iterator and items, is asked of them as they are: `collected_in_impl`,
which put them in the impl's terms, is gone, and its mutation,
`in-place-default-in-trait-terms`, with it. No case's JS changed, as a
program or a library (case N).


## Amendment: a copied default's types are asked as they are (2026-11-06, #809)

`impl_type`, which normalized a type in the impl's typing environment
where a copied default asked one, its drops', a lost `dyn`'s drop's and its
`Iterator` dictionary's, is the identity since #803 put its MIR in the
impl's terms, normalized there: made the identity, no case's JS changed,
as a program or a library, nor any lowering test. It's gone, with
`default-evidence-trait-terms`, which made it so and survived.

## Amendment: what a caller gives of a default's clause it can't meet, and a copied default's closures (2026-11-06, #828)

Since #798 an impl's slots include those each kept default's where clause
names. Generic code that calls one of a blanket impl's defaults was then
asked to give the slot of another default's clause it can't meet there,
`<<I::IntoIter as Iterator>::Item as IntoIterator>::Item` of itertools'
`intersperse`, and was refused. Such a type can't be normalized where
the clause doesn't hold, so no value of it exists there and nothing called
there reads its slot. `drops::no_type_here` is the one answer, of the
concrete case #798 made (`<u8 as IntoIterator>::IntoIter`) too: its drop
is nothing (`DropQuery`) and its facts aren't given (`gives_slot`). It
says so only of type parameters the environment bounds. A dictionary's
method's own `I`, read where its impl's bounds hold, is none of its own,
and its slot, `from_iter::<I>`'s in-place buffer (#794), is still given.

A fact of an impl's slot is found by the slot's type as the bounds here
normalize it (`given_fact_of`), so a copied default whose where clause
says `Self: Iterator<Item = T::Item>` finds the `<Self as Iterator>::Item`
slot it was given as `<T as TupleCollect>::Item` (itertools'
`collect_tuple`). `pattern_paths` follows `elt @ Some(_)`, a binding whose
subpattern moves nothing, as the whole moved into `elt`. A closure made in
a copied default is read with the default's arguments and its own generics
past them, its kind, signature and captures, as they are. Before this,
rust-js crashed on itertools' `minmax_by`'s `|_| ()`, "type parameter out
of range when instantiating".

Held by `blanket_default_beside_an_unmet_clause.rs`,
`default_matches_its_tuple_whole.rs` and
`copied_default_closure_of_its_own_generics.rs`, both builds; mutations
`impossible-alias-unsupported` (retargeted), `unmet-generic-alias-unsupported`,
`unmet-slot-fact-given`, `binding-at-whole-unfollowed`,
`given-fact-unnormalized` and `copied-closure-own-generics-dropped`.
itertools still doesn't compile. Every other default of its blanket impl
is lowered too, and about ten of them are refused (#828).

## Amendment: what a copied default's THIR makes is asked in its impl's terms (2026-11-06, #828)

Since #803 a copied default's MIR, its environment and its drops are its
impl's, but the walk of its THIR that finds what its body does with values
that drop (`drops/facts.rs`, ADR 0098) still read its trait's types: of
itertools' `sorted_unstable`, `Vec::from_iter(self)`'s `Vec<<Self as
Iterator>::Item>`, whose drop no slot of the impl's names, was refused as "a
value of an associated type, where a type may have a destructor", and as a
library itertools didn't compile. Case C of ADR 0262: each item drops as
Rust drops it, by the drop its caller gives.

- **Every type a THIR walk asks the drops of is put in the body's terms by
  one answer, `DropQuery::thir_ty`**: of a copied default, or a closure made
  in it, its type instantiated with `traits::default_body_args`, the
  default's arguments and a closure's own generics past them, the one
  mapping `mir_closure_fn` reads its closure's MIR with too; an alias in
  the impl's terms is normalized where its drop is asked, as any is. The
  facts' `holds_drops`, `taken`, a closure's and a guard's drops, a lost
  `dyn`'s, and `pattern_paths`' bindings ask it.
- What a library's copied default moves of its items a way MIR's lowering
  doesn't follow, `let [_, b] = pair;` of a `[Self::Item; 2]`, is refused
  as a generic function's is, where in its trait's terms it compiled
  (`test/diagnostics.test.ts`).

Held by `copied_default_asks_its_drops_in_the_impls_terms.rs`, both builds,
with native Rust, and mutations `thir-types-in-trait-terms`,
`thir-args-of-no-copied-default` and `pattern-types-in-trait-terms`;
`copied-closure-own-generics-dropped` moved to `default_body_args`.
itertools 0.14 compiles again (docs/crate-corpus.md); its probe is still
refused, first at "an iterator's own `last` given a dictionary of its own
type parameter", as before #828.

## Amendment: an array's parts a copied default moves are followed (#1109)

`let [_, b] = pair;` of a `[Self::Item; 2]` in a blanket impl's copied
default was refused as moving part of a value with a destructor, "as a
generic function's is"; but a generic function's, a concrete one's and a
non-blanket impl's copied default's all compiled and ran as native Rust
does, as MIR's lowering moves each part of an array out. Only the THIR
walk of a blanket impl's copy asked `pattern_paths`, whose `_` arm
refused an array's pattern. `paths_in` follows an array's pattern now,
each element by its place and a subslice, `rest @ ..`, after its prefix,
in a tuple's or a struct's field too, so the copy compiles as the generic
function does. What's left of the `_` arm binds nothing by value rustc
lets move, and a binding's subpattern, `elt @ Some(_)`'s, moves nothing
either, as rustc forbids it to: so `moves`, which asked what a pattern
moves in its trait's terms beside the copy's (`pattern-types-in-trait-terms`,
#1109's survivor), decided only that refusal, and goes with it.
`test/diagnostics.test.ts`'s library copied default moving part of its
items compiles now. Held by `array_parts_moved_in_a_copied_default.rs`, a
blanket impl's `[_, b]`, `[_, rest @ ..]`, a tuple's `(first, [_, label])`
and a struct's `Labels { tag, rest: [_, last] }` of items with a
destructor, and a `match` whose arms move part of a value and all of
it, which the THIR walk refuses and MIR's lowering, a body's own, does
right, against native Rust in both builds; mutations
`array-pattern-parts-unfollowed`, and `binding-at-whole-unfollowed`
refitted. `closure-problems-unfiltered` (`drops/parts.ts`), whose catch
was an array's pattern, catches that `match` now.

## Amendment: a copied default's type, in its impl's terms, is normalized where its drop is asked (#1109, #1337)

Case ([0262](0262-when-rust-and-js-disagree.md)): none for results. The
facts were wrong, and the outputs right by luck. `DropQuery::thir_ty`
instantiated a type a copied default's THIR names with its impl's
arguments, but didn't normalize it. `Vec<<Wrapper<I> as Iterator>::Item>`
of a non-blanket impl was then asked as that alias, which the drop slots
take for one dropping nothing. The blanket impl's `Vec<<T as Iterator>::Item>`
drops. So a part-and-whole `match` of a pair of them compiled in the
`Wrapper<I>` impl, agreeing with native Rust only by luck, while the blanket
impl's was refused, "moving part of a value with a destructor".

**What a copied default's THIR names is normalized in its impl's terms
before its drop is asked**, as the doc comment always said, so both
impls answer alike and both are refused. Facts must be true; that refusal
is #1337's to lift where MIR's lowering follows the moves (the
coordinator's decision). Corpus case
`copied_default_of_a_wrapper_normalizes_its_items_drop` (compile-fail);
mutation `thir-ty-unnormalized` (`lower/drops/types.ts`).

## Amendment: a `match` that moves a part in one arm and all of it in another is followed (#1337)

Case ([0262](0262-when-rust-and-js-disagree.md)): none. Each part drops
where Rust drops it.

The THIR walk of a copied default refused a `match` of a value with a
destructor whose arms moved parts of it, `(front, _) if .. =>`, wherever
another arm moved all of it, `both =>`: "moving part of a value with a
destructor". The arms' moves are what MIR's lowering already follows, as it
does where the copy isn't walked, a concrete function's, a generic one's:
an arm that moves a part leaves the rest to its owner, dropped where the
owner goes, after what follows the `match`; one that moves all of it drops
it where that arm leaves it; a guard binds by reference and moves nothing
where it fails; a panic in an arm drops the part it moved, then what's
left. So the rule is one: each arm's moves are followed, `pattern_paths`,
a whole binding's its root's. Corpus case
`match_moves_a_part_or_the_whole.rs` (a blanket impl's copy and a
`Wrapper<I>` impl's, generic functions' beside them, a guard that fails
after binding, a panic in an arm) and
`copied_default_of_a_wrapper_normalizes_its_items_drop.rs`, refused before,
against native Rust in both builds; mutation
`match-whole-arm-beside-a-part-refused`.
