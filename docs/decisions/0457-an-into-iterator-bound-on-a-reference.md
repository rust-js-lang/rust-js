# 0457. An `IntoIterator` bound on a reference is given its dictionary

Status: Accepted. Extends [0061](0061-generic-iterators.md),
[0160](0160-user-collections.md) and
[0454](0454-a-mut-given-as-a-type-parameter.md).
Amended by [0976](0976-an-into-iterator-parameter-is-held-as-it-is.md): a
`U: IntoIterator` given a reference holds it as it is, never in a box.

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
