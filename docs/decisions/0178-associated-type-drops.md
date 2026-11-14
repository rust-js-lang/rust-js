# 0178. An impl's dictionary has its associated types' drops

Status: Accepted. Amended last: an `impl Trait` of generics its drop can't
read (#1575). Extends [0098](0098-destructors.md),
[0106](0106-generic-traits.md) and [0163](0163-trait-method-drops.md).

Case: C, D ([0262](0262-when-rust-and-js-disagree.md)).

## Context

Generic code can hold a value of an associated type, whose type only the
impl knows:

```rust
pub struct DateTime<Tz: TimeZone> {
    datetime: NaiveDateTime,
    offset: Tz::Offset,
}
```

Dropping one runs the impl's type's destructor, if it has one. A type
parameter's drop is given by its caller (ADR 0098), but nothing gave one
for an associated type, so holding one was an error in any crate where a
type may have a destructor: one of its own, or any library's. That stopped
chrono, at 55 places.

## Decision

**An impl's dictionary has a drop for each associated type whose type has
a destructor, `$dropOffset`, and generic code calls it, if it's there:**

```js
function describe(zone, ZZone) {
  const first = ZZone.offset(zone, "first");
  try { .. } finally { ZZone.$dropOffset?.(first); }
}

$loudZone = { offset: loudZone_offset, show: loudZone_show, $dropOffset: noisyDrop_drop };
```

- **Beside `$drop`,** the dictionary's own type's drop: a name no Rust
  method can have.
- **A `dyn`'s dictionary has the drop of the type it names,**
  `dyn Source<Item = Noisy>`'s `$dropItem`.
- **A library's generic code calls it though the library has no
  destructor:** its consumers may, as a generic method's drops are given
  (ADR 0163).
- **A std trait's associated type, an iterator's `Item`,** has no
  dictionary: its drop is its caller's to give, as a type parameter's is
  ([ADR 0421](0421-std-associated-type-drops-are-given.md)). (Amended: it
  was an error where a type may have a destructor.)

## Why

- **It's exact:** the `associated_type_drops` corpus case drops a value of
  an associated type in generic code, in a generic struct, in an `Option`,
  at the end of a block, on reassignment and through a `dyn`, of an impl
  with a destructor and one without, with native Rust; the crates test
  drops a consumer's value in a library's generic code.
- **It's what a type parameter's drop is:** the same `?.()`, from what the
  caller already gives.

## Amendment: what the dictionary made a JS iterator of is what it drops (#978)

An associated type bound by `Iterator` that an impl gives the crate's own
iterator is a JS iterator in generic code, `$iterator(value, next)`, made
by the dictionary's method (ADR 0061), but its `$dropIter` dropped what it
was given as the struct itself: `drop at undefined` where native Rust shows
the struct's state. The two read one answer, `item_made_js_iterator`: the
drop is `held_drop_function`'s of an iterated value, which drops its `of`.
And generic code drops one as it drops a type parameter's value given its
caller's drop (`has_generic_drop`): after a consumer that doesn't close it,
`collect()`, and closed with the adapters that hold it, `rev()`; a callee
given such a type is given its drop, where an impl gives the type one whose
drop runs (`item_may_drop`). Corpus case `associated_iterator_dropped`.

## Amendment: a trait method's `impl Trait` is dropped by its dictionary too (2026-11-11, #1296)

Case ([0262](0262-when-rust-and-js-disagree.md)): a refusal lifted;
results kept as Rust's.

What a trait method gives behind an `impl Trait`, `fn hidden(n: u32) ->
impl DoubleEndedIterator<Item = u32>`, is rustc's anonymous associated
type of its trait. Generic code holding one and dropping it,
`let mut it = S::hidden(3); it.next_back()`, was refused wherever a type
of the crate may have a destructor ("a value of an associated type,
where a type may have a destructor"), in a library and in a program. No
dictionary carried its drop: `item_types` and `item_drop_of` left out an
`impl Trait`, and `item_drop_key` read an item's name, which an
anonymous associated type hasn't.

A dictionary now carries an `impl Trait`'s drop as it carries a named
associated type's. Its key's word is its method's name, capitalized as
an associated type's name is, so it reads as `$dropItem` does:
`$dropHidden`, `$dropLoud` (`item_word`, traits/words.rs, which
`item_fact_key` builds `HiddenConstSize` of for #1312; m6-react's
decision for the user, 2026-11-11). Generic code reads it as any item's,
`SSource.$dropHidden?.(it)`. An impl whose value drops nothing has
none. Of std's compiled struct, which a dictionary makes the JS iterator
it is, `$knowing(struct, ..)` (#1292), the drop is its own,
`$dropHidden(map) { map.return(); }`, not #978's `it.of`, which only an
`$iterator(value, ..)` has. That drop threw "reading 'return'", and the
same was latent in #1292's `type Iter = Map<..>` wherever its closure
holds a value with a destructor (`held_drop_function`).

Left out: a method with more than one `impl Trait` in its signature, `->
(impl A, impl B)`, whose two would share a word. Its value is refused as
before, now naming #1320 (`one_impl_trait_of`, `shares_its_word`). A
method with generics of its own, whose `impl Trait`'s entry is a function
of them, is #1312's.

`test/corpus/impl_trait_of_a_trait_method_dropped_by_its_dictionary.rs`
(a compiled `Map` whose closure holds a `Loud`, an `impl Sized` that is
one, and an impl of neither) runs against native Rust in both builds and
as a library. `test/crates.test.ts`'s "a library's generic code drops a
consumer's impl Trait by its dictionary" covers a library's generic code
and a consumer's impl. `test/corpus/impl_traits_beside_another_of_a_trait_method.rs`
holds the #1320 refusal. Mutations: `impl-trait-item-uncarried`,
`impl-trait-item-drop-unasked`, `impl-traits-beside-another-unnamed`,
`compiled-struct-dropped-as-made-of` and `impl-trait-word-uncapitalized`;
`operator-output-drop-unseen` refitted. The same refusal of the
neighbours net, `associated_iterator_of_stds_struct_from_its_back.rs`
beside `compiled_adapters_closed_as_a_collect_unwinds.rs` in the build of
std's structs, whose neighbour made the drop asked (#1319), runs as native
Rust, its line off `test/neighbours-known.txt`.

## Amendment (2026-11-12, #1510): std's `IntoIterator` dictionaries carry them too

Case ([0262](0262-when-rust-and-js-disagree.md)): results change; Rust's
behavior preserved. Every impl's dictionary carried its associated types'
drops, `$dropIntoIter`, but std's own `IntoIterator` dictionaries, built by
`made_steps`, didn't. Generic code holding what one makes drops it by that
entry: std's compiled `Flatten`, in `Option::insert` of its next inner
iterator, dropped the one it ran out by `$dropIntoIter?.(..)`, which was
undefined, so `vec![c("a1"), c("a2")].into_iter().flatten().nth(4)` in the
std build never dropped `a1` where native Rust drops it before `Some(1)`.
`made_steps` now adds `item_drops`, of the value itself where the
dictionary gives what it makes as it is (ADR 0552, `as_it_is`), else as
before. 27 snapshots' dictionaries gain `$dropItem` and `$dropIntoIter`
where those types drop. Corpus case `flatten_nth_drops_each_inner_it_runs_out`
against native Rust in both builds, and in iterator-forms' few; mutations
`into-iterator-dictionary-drops-nothing` and
`into-iterator-as-it-is-dropped-as-made`.

## Amendment (2026-11-13, #1554): a serde backend's associated types drop nothing

Case ([0262](0262-when-rust-and-js-disagree.md)): none observable; a
refusal lifted. An associated type of a serde trait no crate may
implement, `<D as Deserializer<'de>>::Error`, `<A as SeqAccess>::Error`,
`<S as Serializer>::Ok`, was "a value of an associated type, where a type
may have a destructor" wherever the crate had a destructor, and always in
a library, which compiles a generic `deserialize_with` helper as it's
written: 62 of #1532's serde probes refused as libraries only. A crate's
own `Deserializer`, `Serializer`, access or `de::Error` is refused
(`implementable`), in a library's consumer too, so their impls are
serde_json's alone, values rust-js makes that hold none of the program's
(ADR 0462, `is_json_error`): they drop nothing, whichever backend a
caller gives, a fact of recognition (`of_serde_backend`), not a caller's.
A `Visitor`'s `Value`, the crate's own, is asked as before. Native-compared
in test/serde.test.ts as a program and as a library; mutation
`serde-backend-type-may-drop`.

## Amendment (2026-11-14, #1575): an `impl Trait` of generics its drop can't read

Case ([0262](0262-when-rust-and-js-disagree.md)): none; a refusal lifted,
each drop as Rust's.

Left out above, a trait method's `impl Trait` of generics of its own,
`hidden<const M: u32>`'s or `tagged<U: Copy>`'s, was refused wherever a
type may have a destructor, "a value of an associated type": its type is
one of each instance, so no one drop of it was carried. A neighbour with a
destructor beside a case of these, `btree_append_drops.rs`, refused it (the
neighbour shuffle). But where each of those generics is one its drop can't
read, a lifetime, a constant, or a type bound by `Copy`, which has no
destructor, nor holds what has, the drop is the same of every instance.
So the dictionary carries it as of one of no generics: `$dropHidden`, made
of its type for the impl with `()` and zero for those generics
(`drop_params.rs`'s `generic_impl_trait_types`), and generic code reads it
(`item_drop_of`). One rule says which, `drop_carried_impl_trait`
(`traits/words.rs`), which the dictionary and the reader both ask, and
which takes in the case of no generics of its own. A type parameter not
bound by `Copy`, whose instances may each hold a destructor of their own,
stays refused (#1312).

Held by `test/corpus/impl_trait_of_a_method_with_its_own_generics_dropped.rs`
(a `U: Copy` and a `const M` method each holding a `Loud`, held and dropped
by generic code, against native Rust, in both builds), the diagnostics'
refusal of a non-`Copy` one, #1575's neighbour pair off
`test/neighbours-known.txt`, and the mutations `impl-trait-of-any-generic-carried`,
`impl-trait-of-own-generics-uncarried` and `generic-impl-trait-drop-unmade`.

