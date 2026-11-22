# 0457. An `IntoIterator` bound on a reference is given its dictionary

Status: Accepted. Extends [0061](0061-generic-iterators.md),
[0160](0160-user-collections.md) and
[0454](0454-a-mut-given-as-a-type-parameter.md).
Amended by [0976](0976-an-into-iterator-parameter-is-held-as-it-is.md): a
`U: IntoIterator` given a reference holds it as it is, never in a box.
Amended by [0552](0552-an-into-iterator-held-as-it-is-is-given-as-it-is.md)'s
#2331 amendment: what the dictionary's `into_iter` makes is its type's own
value, stepped by its `IntoIterIterator` entry, `for (const x of
$iterator(C.intoIter(c), C.IntoIterIterator().next))`.
Amended (#2336): how generic code holds the reference is `ref_held`'s
one answer; a `&mut I` it holds as concrete code does is stepped as
`&mut I`'s own.

Case: N ([0262](0262-when-rust-and-js-disagree.md)): the same items, and
what's written through a `&mut` one is written to the collection.

## Context

Generic code that keeps a collection and iterates it through a reference
bounds the reference: either's `iter_mut`, `for<'a> &'a mut L:
IntoIterator`, and code over any collection, `for<'a> &'a C:
IntoIterator<Item = &'a u32>`.

A `&C` there was read as a JS iterable, `Iterator.from(c)` (ADR 0061).
That can't serve a `&mut C`: its items are handles on the collection's
places, and which handles, a `Vec`'s items, a map's values or an
`Option`'s one, only the caller knows. It was refused, and either with
it. A crate's own collection was refused even through a `&C`, as one
given where any `IntoIterator` goes (ADR 0160).

## Decision

**An `IntoIterator` bound on a reference to a type only a caller knows,
`&C` or `&mut C`, is given a dictionary, `{ into_iter }`, and generic
code's `into_iter` of it, `for x in c` too, is the dictionary's. One on a
type parameter itself, `I: IntoIterator`, is given the JS iterator, as
before.**

```rust
fn double_all<C>(c: &mut C)
where
    for<'a> &'a mut C: IntoIterator<Item = &'a mut u32>,
{
    for x in c {
        *x *= 2;
    }
}
```

```js
function double_all(c, CRefMutIntoIterator) {
  for (const x of CRefMutIntoIterator.into_iter(c)) {
    x.value = Math.imul(x.value, 2) >>> 0;
  }
}

double_all(c, { into_iter: (cell) => $iter($mutItems(cell.value)) });
```

1. **One answer, `into_iter_by_dictionary`**, of a bound's trait and its
   self type. `has_dictionary_of` asks it for every bound: which bounds a
   function takes dictionaries for, and which associated types' drops
   are its own parameters. A call of `into_iter` asks it in the impl's
   terms, `known_trait_call`, where a copied default's `&Self` is a
   `&[T]` that std's impl answers. A collection of the crate's given for
   one isn't refused.
2. **std's entry is what `into_iter` of the reference is where its type
   is known**: a handle on each item of a `&mut`, `$mutItems`,
   `$mutEntries`, an `Option`'s place; a `&`'s, std's own iterator of
   it. It's given what generic code has, a box of a `&mut` to an object,
   `cell.value` (ADR 0454), asked of `held_in_box`.
3. **The crate's is its impl's**: `impl IntoIterator for &mut Bag`'s
   method, given the `Bag` in that box. It's written only where generic
   code reads it.
4. **Its evidence is named for the reference**, `CRefIntoIterator` and
   `CRefMutIntoIterator`, as an array's is `TArray`.
5. **A bound's own `&mut C` isn't a value**: a caller given a `&mut Vec`
   for it isn't refused as one inside generic code's parameters (ADR
   0454), as a `D: Trait`'s `D` isn't.

## Why

- **Only the caller knows how to iterate it**: one dictionary, the form
  every other bound has, where a JS iterable can't hold a `&mut`'s
  handles.
- **One form for both references**: a `&C` and a `&mut C` read the same,
  as the user chose; the crate's collections work through either.
- **It's tested**: a corpus case doubles a `Vec`, an array and an
  `Option` through a `&mut C`, sums them through a `&C`, does both to a
  crate's collection, bumps a map's values, and steps `&mut it` of an
  `I: Iterator`, std's own, against native Rust; a mutation of each part
  is caught.

## Alternatives

- **A dictionary for `&mut C` alone**, `&C` read as a JS iterable: no new
  argument where it works today, but two forms for one kind of bound,
  and the crate's collections still refused through a `&C`.
- **Generic code choosing the handles by the bound's `Item`**: `$mutItems`
  of a `&mut u32` item. Wrong for an `Option`, whose one item is its own
  place, and for a crate's collection.

## Consequences

- A function with such a bound takes one more argument: `total(v, {
  into_iter: (x) => $iter(x) })` where it was `total(v)`.
- either compiles.

## Amendment (2026-11-05): a `&mut` of an iterator is std's every iterator's (#742)

Case ([0262](0262-when-rust-and-js-disagree.md)): none; valid Rust was
refused with std's iterator structs. `vec![&mut a, &mut b].into_iter()
.flatten().count()`, of the crate's iterators, and `(0..5).zip(&mut c)`
and `(10..12).chain(&mut d)` asked std's `IntoIterator` of a `&mut Own`,
which went to this decision's dictionary of a collection's `&C` or `&mut
C` by its type's shape, a reference, and had none: "implementation
evidence for `<&mut Own as std::iter::IntoIterator>`". rustc selects std's
impl for every iterator there, `impl<I: Iterator> IntoIterator for I`, of
`I = &mut Own`.

**What a reference's `IntoIterator` is follows rustc's selection**:
`ref_into_iter_of_collection` is the one answer, of a bound's dictionary and of
a type parameter given one, `zip`'s and `chain`'s `U` (ADR 0976). A
reference is a collection's where std's blanket impl isn't the one
selected (`iterates_as_itself`); one where it is, a `&mut` of an iterator,
is given as it is, ADR 0552's dictionary, its steps the `&mut I`'s.
`test/corpus/mut_ref_iterators_into_iter.rs` checks a flatten, a zip and a
chain of `&mut`s of the crate's iterator, beside a `Vec`'s `&mut` and `&`,
against native Rust in both builds; the mutations are
`ref-iterator-into-iter-by-collection` and `ref-iterator-not-itself`
(`lower/traits/dictionaries.ts`).

## Amendment (2026-11-23): how generic code holds a reference is one answer (#2336)

Case ([0262](0262-when-rust-and-js-disagree.md)): results change, so
Rust's behavior is preserved. `fn two<C>(c: &mut C) where for<'a> &'a mut
C: IntoIterator<Item = u32>` given `&mut range` threw a TypeError. The
caller crosses a `c: &mut C` of a `C` no bound says steps into generic
code's box, `$cellOf(r)` (`Form::Box`, ADR 0454). Its dictionary of
std's `impl IntoIterator for I` of `I = &mut Range` was `{ intoIter:
(items) => items, IntoIterIterator: () => ({ next: $next }) }`, which is
right only where generic code holds the `&mut I` as the JS iterator its
caller made, as a `T: IntoIterator` or a `C: Iterator` given one does
(ADR 0061). Generic code stepped the cell. The new case's metamorphic
rewrites found three more shapes of this, all of which failed on main too:
a `T` of `t: impl Is<T>` (#966), a `T: IntoIterator` given as it is that
steps what it makes, and a `&mut C` whose `C` the callee takes as it is.

**How generic code holds a reference its caller gives is one answer,
`ref_held` (generic_facts.rs), for a collection's `&C` or `&mut C` and
for an iterator's `&mut I` alike.** It returns one of three:

- `InBox`: the box its caller makes of a `&mut` to an object, a `c: &mut
  C`'s (`form_differs` for `Form::Box`).
- `AsItIs`: as concrete code has it. This covers a type parameter given
  as it is, a `&mut` of one, a collection's given to a type parameter
  (`zip`'s `U`, ADR 0976), and a type parameter another bound names,
  `impl Is<I>`'s, whose value arrives as that one's
  (`ref_held_by_a_parameter`, #966).
- `Stepped`: the JS iterator its caller made of an iterator given to a
  type parameter (ADR 0061).

`RefHeld::read` gives the value back as the caller has it, through the
same box read as `form_back`. dictionaries.rs's `held_in_box` of this
dictionary is gone: a dictionary made with no crossing, which is where a
collection's `&mut C` and a type parameter another bound names get theirs,
asks `ref_held_by_a_parameter`, the answer for a type parameter given the
type. calls.rs's `as_it_is` keeps only the routing: a collection's given
to a type parameter of its own is made there, unkept, as before, with
`ref_held`'s answer. Every collection's snapshot is byte-identical.

The dictionary for anything but `Stepped` is `ref_into_iter_dictionary`'s.
Of a collection it's as before. Of a `&mut I` it gives the `&mut` as its
caller has it, stepped by `&mut I`'s own `Iterator` with `I`'s steps
passed on (`mut_ref_iterator_dictionary`), and it is kept once per
function like the others (`kept`): `two($cellOf(r), { intoIter: (cell) =>
cell.value, IntoIterIterator: () => ({ next: $rangeNext }) })`. A
function's own dictionary of a `&mut C` it holds as it is (a `C:
Iterator`'s), passed on to a callee that boxes it, gets its `into_iter`
given what the box holds: `$throughBox(CRefMutIntoIterator, { intoIter:
[1] })`.

`test/corpus/mut_ref_iterator_through_a_reference_bound.rs` checks, in
both builds against native Rust:

- a range, a `vec::IntoIter`, the crate's iterator, and std's `map` and
  `copied` adapters;
- a `C: Iterator` passing its own on, and an `I: Iterator` forwarding
  `&mut i`;
- a sum and a `while let`;
- a `T` of `impl Is<T>`;
- a `T: IntoIterator` given as it is that keeps what it makes.

Its controls, a `T: IntoIterator` and a `C: Iterator` given the `&mut`,
are in the same case. The mutations are:

- `iterator-ref-stepped-as-js` (`lower/traits/dictionaries.ts`);
- `ref-held-box-unread`, `ref-held-as-it-is-taken-for-js`,
  `named-into-iter-unboxed` and `into-iter-param-boxed`
  (`lower/generic_facts.ts`; the last two retargeted from calls.rs);
- `own-mut-ref-into-iter-unboxed` (`lower/traits/calls.ts`).

## Amended by #803

`known_trait_call`'s two exceptions are gone with it: a copied default's
body is in its impl's terms (ADR 0495, amended), so what recognition knows
of an `into_iter` call is `classify`'s answer alone, the corpus's cases of
this ADR's unchanged.
