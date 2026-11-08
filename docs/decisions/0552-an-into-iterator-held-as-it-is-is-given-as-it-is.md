# 0552. An `IntoIterator` whose iterator is held as it is is given as it is

Status: Accepted; extended by [0566](0566-fuse-is-stds.md): an `IntoIter`
the signature holds in a struct is held as it is too, and the
`IntoIterator` dictionary carries its `Iterator`, which steps it. Amends
[0061](0061-generic-iterators.md); extends
[0411](0411-an-iterator-given-as-it-is-has-its-dictionary.md) and
[0457](0457-an-into-iterator-bound-on-a-reference.md).

Case: C ([0262](0262-when-rust-and-js-disagree.md)): a generic
function's `into_iter()` gives what Rust's gives, the iterator its type's
`IntoIterator` makes.

## Context

A `T: IntoIterator` is given a JS iterator, and `items.into_iter()` is
`$intoIter(items)` (ADR 0061). A type parameter bound by `Iterator` and a
trait that reads its value, `Clone`, is given as it is, stepped and cloned
by its dictionaries (ADR 0411). The two meet where a function holds what
`into_iter()` gives as a type parameter given as it is:

```rust
fn twice<T: IntoIterator>(items: T) -> Twice<T::IntoIter>
where
    T::IntoIter: Clone,
```

`twice(0..3)` is given `$lazyRange(0, 3)`, a JS iterator, but `Twice<I>`'s
impl steps it by `$rangeNext` and clones it as a range, `{ start, end }`:
one value, two representations. itertools' `multi_cartesian_product`
does this of `<H::Item as IntoIterator>::IntoIter`, and `$intoIter` of a
range's `{ start, end }` is no iterator: it gives `[]` where Rust gives
four products. It's the last difference in itertools' probe.

## Decision

**A bound `X: IntoIterator`, of a type parameter or an associated type,
whose `X::IntoIter` the function bounds by a trait that reads its value,
`Clone` or `Debug`, as ADR 0411 reads one, is given as it is, with its
`IntoIterator` dictionary, `{ into_iter }`, which makes the `IntoIter` as
it is.** `items.into_iter()` is `TIntoIterator.into_iter(items)`.

- **One answer, `into_iter_held_as_it_is(id, X)`**, which the bound's
  dictionary, `given_as_it_is`, the `into_iter()` call and what's written
  in place ask. Its value-reading rule is `given_as_it_is`'s own,
  `reads_its_value`, shared, not copied.
- **The caller's dictionary**: of an iterator, a range's too, std's
  `impl<I: Iterator> IntoIterator for I`, `{ into_iter: (items) => items }`;
  of a `Vec`, `{ into_iter: (items) => $iter(items) }`, std's iterator over
  an array as concrete code has it; of one given as it is, the caller's
  own, passed on. A library's bound is read by its consumer from its
  clauses, as rustc has them.
- The caller gives `X` as it is, not made a JS iterator.
- Everything else is ADR 0061's, unchanged: an `IntoIterator` whose
  iterator nothing holds as it is is still given a JS iterator.

## Why

`into_iter_held_as_it_is` makes a `twice` of a range, a `Vec` and a
string's `chars()`, and of each `H::Item` of ranges and of `Vec`s, as
`multi_cartesian_product` does, each stepped and cloned, against native
Rust.

## Consequences

- itertools' probe runs as native Rust does, every line.
- Mutations answer no, leave the parameter a JS iterator, give no
  dictionary, write `into_iter()` as std's or `$intoIter`, and refuse an
  iterator's own `IntoIterator`.

## Amendment: an iterator only bounds know, and an impl's bound

Case: A ([0262](0262-when-rust-and-js-disagree.md)): refusals lifted;
itertools compiles again, which ADR 0566's `Fuse` had stopped.

- **An iterator only bounds know, given where an `IntoIterator` is held as
  it is**, `put_back(j)` of a `j: J::IntoIter`, has std's `IntoIterator`
  for every iterator, `iterates_as_itself`: its evidence is `{ into_iter:
  (items) => items, IntoIterIterator }`, the latter the `Iterator` its
  bounds give. It was refused, as a type only bounds know was taken to
  have none.
- **An impl's bound is its dictionary's**, which each of its methods is
  given: whether its `IntoIter` is held as it is is the impl's answer and
  its methods' alike, any of them holding it. `impl<A: IntoIterator>
  From<(A,)> for Zip<(A::IntoIter,)>`'s `from` holds it in `Zip`; the impl
  alone, no function, said not, so its dictionary took none and its
  method asked one.

`test/corpus/into_iter_of_a_bound.rs` has both, `put_back` and a
`From` impl given to generic code as `multizip` is, against native Rust.
The readability measure's `$iter` share grows, 61.9 to 62.3 in ten
thousand lines: the case gives a `Vec`'s `IntoIterator` dictionary,
`into_iter: (items) => $iter(items)`, this ADR's form, three times.
Mutations `into-iterator-of-bound-iterator-refused` and
`impl-bound-held-by-itself`.

## Amendment: the crate's generic iterator's `into_iter()` is itself (2026-11-06, #806)

Case: A ([0262](0262-when-rust-and-js-disagree.md)): a refusal lifted.
`into_iter()` of an iterator gives it back as it is, so nothing that
runs changes.

Generic code's `for x in w` over the crate's own generic iterator struct,
a `w: Wrapper<I>` of `impl<I: Iterator> Iterator for Wrapper<I>`, was
refused, "calling `IntoIterator::into_iter`", though the same loop over a
concrete `Wrapper<Range<u32>>` compiled. MIR's `into_iter()` took a value
for its own iterator only where `implements_iterator` said so, which
reads the type's `Item` and doesn't count one that's still a projection
of std's `Iterator`, `I::Item`.

Whether `into_iter()` gives a value back as it is is rustc's selection of
std's blanket `impl<I: Iterator> IntoIterator for I`, `iterates_as_itself`,
which the dictionaries already ask (above). MIR's `into_iter()`
(`value_into_iter`) now asks it too, the one answer: the loop steps the
crate's `next`, `wrapperIterator_next(iter, IIterator)`, as a concrete
loop does. No other case's JS changed.

`test/corpus/generic_for_over_own_iterator.rs` loops over a `Wrapper`
whose `next` prints, generic over its iterator, over its items, an impl's
method looping `for _ in self`, and a loop that drops a value and breaks,
against native Rust in both builds. Mutation:
`generic-own-iterator-into-iter-refused`.

## Amendment: an `IntoIterator` another bound names is held as it is (#966)

`fn total<I: IntoIterator>(items: impl Is<I>)` gives its body an `I` no
parameter of type `I` holds: `items.is()` gives what the caller gave as
`impl Is<I>`, its own value, which no caller made the JS iterator of its
items. It was stepped as one, `for (const x of items)`, a `TypeError` of a
crate's iterator or a `Vec`'s `IntoIter`. `into_iter_held_as_it_is` answers
yes of a type parameter another bound names (`named_by_another_bound`, the
one answer `given_as_it_is` reads too, ADR 0411), and the parameter that
names it is given as it is with it, one value, one representation. Its
items are its dictionary's `into_iter`'s wherever it's stepped, a
collection's `extend` too (`items_by_dictionary`). A `&mut` collection
given so arrives as the naming parameter's value, in the box generic code
holds a `&mut` to an object in (ADR 0454), so its `IntoIterator` reads it
there, not as ADR 0976's `zip` takes a `&mut [T]` as it is. A `&mut` to an
iterator given so is still read as the iterator (#966's
`iterables_given_to_generics`). Corpus case
`into_iterator_named_by_another_bound`.
