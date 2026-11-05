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
