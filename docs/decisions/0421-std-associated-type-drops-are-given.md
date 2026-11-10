# 0421. A std trait's associated type's drop is its caller's to give

Status: Accepted. Extends [0098](0098-destructors.md), whose type
parameters' drops a caller gives, [0163](0163-trait-method-drops.md) and
[0178](0178-associated-type-drops.md), and amends
[0061](0061-generic-iterators.md). Amended by
[0489](0489-a-yielded-item-is-given-its-drop-by-its-own-type.md): one's
given by its own type, not its type parameter's.

Case: C ([0262](0262-when-rust-and-js-disagree.md)): a value is dropped
where Rust drops it, once.

## Context

Generic code drops values of a std trait's associated type of its type
parameters: an `I: IntoIterator`'s iterator, `I::IntoIter`, when it's done
with it, an `I: Iterator`'s item, `I::Item`, that it doesn't keep.
smallvec's `extend` does, of whatever it's given: another `SmallVec`'s
`Drain`, whose `Drop` puts back what it didn't take.

```rust
fn extend<I: IntoIterator<Item = A::Item>>(&mut self, iterable: I) {
    let mut iter = iterable.into_iter();
    ...
    for elem in iter { self.push(elem); }
}
```

A crate trait's associated type has its drop in its impl's dictionary
(ADR 0178); a std trait has no dictionary, and holding a value of one was
an error wherever a type may have a destructor, and a loop over one took
no destructor of its type parameter (ADR 0190). Their caller knows what
they are: a `vec::IntoIter<Loud>`, a `Drain`.

## Decision

**A generic function is given a drop for each std trait's associated type
of its type parameters, `dropIIntoIter`, as it's given one for a type
parameter, by its caller, of the type it is there.**

```js
function first(items, dropIIntoIter) {
  const it = Iterator.from(items);
  ...
  dropIIntoIter?.(it);
  return result;
}

first(loud, (intoIter) => { for (const item of intoIter) loudDrop_drop(item); });
```

- **Which it's given is its bounds'**, `traits::drop_slot`: after its type
  parameters', each associated type of a bound of a trait no dictionary
  carries, `<I as IntoIterator>::IntoIter`, `<I as Iterator>::Item`, its
  supertraits' too, in its bounds' order. Its callers, a dictionary's, and
  a library's consumers read the same bounds, so they agree, and a
  library's manifest lists them as it lists its type parameters' drops.
- **One a bound names isn't one**: `IntoIterator<Item = A::Item>`'s `Item`
  is `A::Item`, its impl's dictionary's.
- **It's given where its type parameter is** (ADR 0098), and taken only
  where it's used (ADR 0300); a trait's generic method's are its
  declaration's, which a call through a dictionary gives (ADR 0163).
- **An iterator of the crate's given as a JS iterator,
  `$iterator(value, next)`** (ADR 0061), **is dropped as `value`**, its
  `of`: what generic code holds of it is the JS iterator, of a type
  parameter's drop or its `IntoIter`'s, `iterators::iterated_slot`. A
  trait impl's method is given a JS iterator of one as a function is,
  `iterators::given_iterators`.

## Why

- **It's the program Rust runs**: `test/corpus/std_projection_drops.rs`
  drops what's left of a `Vec`'s iterator and of an iterator of the
  crate's with a `Drop`, given to a function, passed on to another, and to
  an `Extend` impl that leaves early, and an `I::Item` it doesn't keep,
  with native Rust; `test/crates.test.ts` a consumer's, in a library's
  generic function.
- **It's a type parameter's drop**: the same `?.()`, given the same way.

## Consequences

- A type parameter given no value with a destructor is given no drop for
  its associated types either: one with none whose iterator has one, a
  `Copy` type's, is an error still.
- An adapter of std's holding an item with a destructor, `take`'s of a
  `repeat`, is closed to drop it (ADR 0422).
- Before, a crate's iterator given as a JS iterator was dropped as the JS
  iterator, which its `Drop` read as its own: a silent difference, now the
  value's.

## Amendment: a drop that may be none is called only where it's there (2026-10-30)

`insert` and `get_or_insert` of an `Option` in generic code dropped the
value they don't keep by the drop they were given, called as though it
were there, `dropT(old)` or, of an associated type's, std's `flatten`'s
front and back iterators, `IItemIntoIterator.$dropIntoIter(old)`: a
`TypeError` where what it's called with has nothing to drop, which neither
is then. Every program reaching it threw: `put(&mut Some(1), 2)`, and
`v.iter().flatten()` stepped from its back of std's iterator structs
(ADR 0908). Results change, so Rust's behavior is kept (ADR 0262): **a
drop that may be none, a type parameter's its caller gives or one an
impl's dictionary holds, is `?.()`**, as this ADR has it, in an expression
too: `drops::functions`' `std_drop_call`, which `insert`'s,
`get_or_insert`'s, `step_by`'s and `inspect`'s drops are. Held by
`option_insert_given_drops.rs` and `flatten_from_its_back.rs` of std's
structs against native Rust and the mutation `given-drop-called-as-there`
(`lower/drops/functions.ts`).

## Amendment: a type parameter's value given as a JS iterator (#766)

Generic code takes a type parameter's iterator as a JS iterator; its
caller makes one of the crate's own, `$iterator(value, next)`, and gives
it a drop of the iterator's `of`, `held_drop_function`'s. Generic code
that passes a value of its own type parameter on to generic code as one
made it by `$given(value, IIterator)`, and gave its own drop of the value
on as it was: std's compiled `FlattenCompat::count` and `last`, whose
inner `count(acc, iter: U)` dropped `$given`'s JS iterator by `dropU`, a
drop of the `U` itself. `vec![once(6)].into_iter().flatten().count()`, of
the crate's iterator with a destructor, threw a `TypeError` in the build
of std's iterator structs, where native Rust drops it (a regression of
#736's f73daa2c5). Results change, so Rust's behavior is kept (ADR 0262).

`iterated_slot` says so of a type parameter given too, and
`held_drop_function`, the one answer of what drops a value generic code
holds, gives it `(it) => $givenDrop(it, dropU)`: the value `$given` made
the JS iterator of, its `of`, or the value itself where `$given` gave it
as it is, its dictionary's steps its own, which no type says. Held by
`flatten_count_takes_each_inner_count.rs`'s `Kept`, counted and its
`last` asked, against native Rust in both builds, and the mutations
`given-param-drop-unwrapped` and `given-drop-of-the-wrapper`.

## Amended (2026-11-06, #718): the drop of a type parameter's value keeps each item's own

`held_drop_function` gave a type parameter's drop as the arrow `(it) =>
$givenDrop(it, dropI)` (#766). `$closing` finds the drop of each item, by
which a `Vec`'s `IntoIter` drops a slice it passes over, as the drop's
`each`, `$dropEach`'s; an arrow has none. So a `Vec`'s `IntoIter` of items
with a destructor given as it is, which `$given` gives its slice's steps
(`$slicing`), dropped nothing its `nth`, `advance_by` or `advance_back_by`
passed over: `self.0.skip(1).map(f).next()` of a `Held<IntoIter<D>>` never
printed `drop 1`, `self.0.step_by(2)` lost every other item's drop, and a
map's `nth_back` its last's. A wrong answer, since 1fa4c15ca (#766).

**`$givenDrop(drop)` is the drop itself**: a function of the value or the
JS iterator `$given` made of it, which keeps `drop.each`. Generic code is
given `$givenDrop(dropI)`, which reads better than the arrow, and
`$closing` passes each item's drop as for any other (`passes_each_owned`).
Case ([0262](0262-when-rust-and-js-disagree.md)): results change, so
Rust's behavior is kept. Held by corpus
`given_iterator_of_droppers_drops_what_it_passes`, against native Rust in
both builds (main prints it without `drop 1`, `drop 2` and `drop 4`), and
the mutation `given-drop-hides-each`; `given-drop-of-the-wrapper` names its
line as it now reads.

## Amendment: a drop slot is keyed by its type, numbered once (#887)

A function's drop slots are numbered one way: its type parameters, then
its associated types in `drop_items`' order (`item_slot`), then the types
built of them. That numbering is `traits/drop_params.rs`'s alone:
`drop_slot` gives an index's type, `slot_index` a type's index, and
`item_slot` and `slot_item` an associated type's place. The analysis of
which slots are given, the manifest's built slots and the type facts each
counted past the parameters themselves, and now ask it.

A body is given its drops by their slots' types, `GivenDrop`s keyed by the
type each drops: a type parameter, an associated type of one, or a type
built of them. The index stays with each as where its callers pass it,
which reports of what the body used read; one given for a while, a
dictionary entry's method's own or a copied default's, has none. Three
numberings had met in one map keyed by `u32`. An associated type's drop
lent to a dictionary entry was keyed `u32::MAX - n`. A copied default's
impl's associated types were renumbered from `u32::MAX / 2`, so that its
own parameters, numbered after the impl's, wouldn't take their place
(#802, #804). And `TypeDrops` kept a second map of each associated
type's index beside them. Keyed by type, no two slots can meet, and a
copied default's own drops are its slots' types in its impl's terms, as
its body names them. The dead `TypeDrops::unsupported`, which nothing
filled, is gone. `test/architecture-traits.test.ts` holds the numbering
to `drop_params.rs` and `u32::MAX` out of the drops' and traits' code. The
corpus is unchanged.

## Amendment: the type given is told by its regions apart (2026-11-10, #1239)

Whether a drop slot's value was made a JS iterator by its caller,
`$iterator(value, next)`, so it's given the drop of the value it was made
of, its `of`, compared the slot's type with what the caller gives for its
type parameter as they were. One had its regions erased and the other
didn't, a closure's in an adapter the crate's own iterator holds, so the
same type was told apart, and the caller gave the value's own drop,
`own[0].return()`, which read the JS iterator as the struct: `Cannot read
properties of undefined (reading 'return')` where native Rust drops each
item (`iterated_slot`). They're compared with their regions erased. Held by
`own_iterator_holding_a_closure_dropped_by_generic_code.rs` and the
mutation `iterated-slot-told-apart-by-a-region`; three of #1239's
`generic-instances` rows run as Rust does.

## Amendment: what's given closed is dropped by its closing (#1286)

Case: C ([0262](0262-when-rust-and-js-disagree.md)): a throw where Rust
runs, so Rust's behavior is kept.

Generic code's `collect()` into its caller's `B: FromIterator`,
`collected<I: Iterator, B>(it: I) -> B { it.collect() }`, gives the
dictionary's `from_iter` what it made of `it`:
`$closing(Iterator.from(it), dropI, false)`, which owns `it`'s drop
(#1239). It gave `dropI` as the drop of `from_iter`'s `I` and
`I::IntoIter` too. Of the crate's own iterator, `dropI` is the drop of
what the caller's `$iterator(count, next)` was made of, `(it) =>
loudDrop_drop(it.of._loud)` (#766). The `$closing` `from_iter` drops
has no `of`, so `collected::<Count, Summed>` printed `summed 3` and then
threw "Cannot read properties of undefined (reading '_loud')", where
native Rust prints `drop x`.

What's given closed is dropped by its closing, `(closing) => {
closing.return(); }`, which drops what it holds once, by `dropI`, as an
adapter's drop does, `(map) => { map.return(); }`, its parameter named for
what it drops as that one's is. `own_evidence` takes `closed`, the type
whose value it's given closed. `collected_into` names it,
`iter_ty`, where `closed_parts` says the value it passes is one. Four
corpus cases' `collected` now give `from_iter` the closing's drop in
place of `dropI`, each running as before against native Rust:
`from_iter_impl_of_dropping`, `from_iter_holds_what_generic_code_gives`,
`iterator_held_in_a_place_dropped_with_it` and
`own_iterator_holding_a_closure_dropped_by_generic_code`.

`test/corpus/own_iterator_collected_by_generic_code_into_own_from_iterator.rs`
holds the crate's own iterator collected whole, stopped early (what's
left dropped where `from_iter` returns), through a `map`, while
`from_iter` panics holding it, and a `Vec`'s `IntoIter`, against native
Rust in both builds. Mutations: `from-iter-given-closed-drops-as-its-type`
and `own-evidence-closed-drop-unseen`; `dictionary-collect-without-own-evidence`
and `dictionary-call-own-drops-missing` refitted.
