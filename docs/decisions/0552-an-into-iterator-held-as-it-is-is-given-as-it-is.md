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

## Amendment: a library's own types hold it too (#1025)

Case ([0262](0262-when-rust-and-js-disagree.md)): R. Results changed:
`into_iter_held_as_it_is` counted a struct, or an `impl Trait`'s hidden
type, as one that holds an `IntoIter` as it is when it was the compiling
crate's own, `is_local()`. A library's `both<I, J>(..) -> Both<I::IntoIter,
J::IntoIter>` took `IIntoIterator` and `JIntoIterator` in its own compile,
where `Both` was local, and its consumer, where it isn't, gave none:
`IIntoIterator.intoIter is not a function`. The rule now asks of a type of
the function's own crate, `adt.did().krate == id.krate`, a pure answer of
its signature that the library and its consumers read alike; a crate's
own function is asked as before. #1025's four library rewrites run as
native. Pair test `library_into_iter_held_as_it_is`; mutations
`into-iter-held-in-a-library-type-unseen` and
`into-iter-hidden-by-a-library-unseen` (`lower/iterators.ts`).

## Amendment: a reference to a type parameter given as an `IntoIterator` (#1264)

Generic code given a `&'a I` bounded `&'a I: IntoIterator` holds its
caller's own reference. No caller makes a JS iterator of it, as callers do
of a type parameter they give, so the function is given its dictionary,
`IRefIntoIterator`. `for` and `chain` stepped it right by chance. `zip`,
`eq`, `cmp`, the comparisons and `extend` took the value itself.
strsim's `jaro` zipped a newtype's text and gave 1 where native gives
0.944. Results changed, so Rust's behavior is kept (ADR
[0262](0262-when-rust-and-js-disagree.md)'s third case).

`items_by_dictionary` now makes such a reference's items by its
dictionary's `into_iter`, `IRefIntoIterator.intoIter(word)`, the JS iterator
it makes. `iterable_items` asks it, as `zip`'s, `chain`'s and `extend`'s
arguments do. `js_iterator_of` asks it too, the one maker of what `eq`,
`cmp` and the comparisons step. Its `IntoIter` isn't held as it is, so a
type its own `into_iter()` call makes keeps its form.
`test/corpus/into_iterator_of_a_reference_given_to_generic_code.rs` holds
`zip`, a zip of a zip, `std::iter::zip`, `chain`, `eq`, `ne`, `cmp`,
`partial_cmp`, `lt`, `extend` of a `Vec`, a `VecDeque`, a `BTreeSet` and a
`String`, `splice`, `collect` and `for` against native Rust, in both
builds. In the build of std's iterator structs, std's `zip` asked its `U:
IntoIterator`'s dictionary of a `&I` as of a std collection's reference;
`ref_into_iter_of_collection` now leaves out one a bound of the function's
gives. `Vec::from_iter` of it stays refused, as it was, and so does
`flat_map(|_| word)` in that build, which asks a constant size of `&I` no
caller is given.

## Amendment: `from_iter` of such a reference (#1272)

`Vec::from_iter(r)` and `String::from_iter(r)` of a `&'a I` bounded
`&'a I: IntoIterator` were refused ("calling
`std::iter::FromIterator::from_iter`"), where `r.into_iter().collect()`
compiled. `collected_from_iter` asked for `into_iter` of `&I` as of a
std type, and no caller makes a JS iterator of a bound's reference.
Rust compiles it, so refusing it wrongly rejects valid Rust (a compiler
gap, not a Rust/JS difference: ADR [0262](0262-when-rust-and-js-disagree.md)
doesn't apply). `collected_from_iter` now steps such a reference by its
dictionary's `into_iter`, through `items_by_dictionary`, as `zip` and
`extend` of it do above. It then collects the way `collect()` of that
iterator does:
`Array.from($roomForHint(IRefIntoIterator.intoIter(r), 4, false))`, and a
`String`'s `.join("")`. `test/corpus/from_iter_of_a_bound_reference.rs`
holds both, of a newtype's `&Word` and of a `&Vec`, against native Rust
in both builds. `flat_map(|_| r)`, whose constant size of `&I` was
refused, is ADR [0899](0899-a-flatten-held-is-stds.md)'s (#1273).

## Amended: an impl whose type holds it (#2263)

An impl's own type, or its trait's arguments, can hold what its
parameter's `into_iter()` makes, `impl<A: IntoIterator> FromT<A> for
One<A::IntoIter>`. That impl holds it as it is, as a method whose
signature does: `into_iter_held_as_it_is` asks an impl's type and its
trait's arguments with the same `holds`, where it asked only a
function's signature. The impl then takes `A`'s `IntoIterator`
dictionary, and its dictionary is a `$traitImpl` maker given each
caller's, `oneFromTA(countIntoIterator)`. Before, the impl took none, so
its dictionary was one module const that gave its method, which takes
one as its `-> Self` holds it, a JS iterable's dictionary whatever `A` a
caller picked. A tuple struct's iterator, `one(Count(3))`, was then
stepped as the array of its fields: `Some(0) Some(1)` where Rust gives
`Some(4) Some(5)`, a compiler bug, not a Rust/JS difference (ADR
[0262](0262-when-rust-and-js-disagree.md) doesn't apply). A fieldless
enum's or a unit struct's, which JS holds as no object, is refused by
#2255's rule for a value generic code holds in a place of its own.
`test/corpus/impl_dictionary_over_an_into_iterator_its_type_holds.rs`
holds the impl's dictionary for the crate's iterator, a `Vec`, a direct
call, and a method that holds nothing, against native Rust in both
builds.

## Amended: given back, taken, and made by a dictionary (#2331)

Case: C ([0262](0262-when-rust-and-js-disagree.md)): results changed,
so Rust's are kept.

A generic function that gave `T::IntoIter` back to concrete code,
`fn it<T: IntoIterator>(t: T) -> T::IntoIter`, took `T` as a JS iterator
and gave back `$intoIter(t)`. Concrete code read that as the type it is,
so `it(3..6)` printed `undefined..undefined` where Rust prints `3..6`.
The same happened in an `Option`, a tuple, a `Vec` or a `Box` given back.
An `IntoIter` taken as an input, `it: T::IntoIter`, was a `TypeError`:
the caller gave its range object, and generic code stepped it as a JS
iterator. Even where `T` was held as it is, the crate's own impl's
dictionary was wrong. `impl IntoIterator for Bag { type IntoIter =
Range<u32> }` made a JS iterator of its range, `$iterator(range,
$rangeNext)`, because ADR 0457's code over a reference stepped the same
dictionary's value with `for...of`. That was one value with two
representations.

**What a dictionary makes is its type's own value. A function holds its
`IntoIter` as it is wherever its caller reads it as that type.**

- **The signature.** `into_iter_held_as_it_is` asks one walk,
  `holds_readably`. It answers whether a signature type holds a value
  where its caller reads it as the type it is: itself, `-> T::IntoIter`
  or `it: T::IntoIter`, or inside what holds a value as it is, such as a
  tuple, an `Option`, a `Vec`, a `Box`, a type of the crate's or a
  compiled std struct. It reads what an `impl Trait` of the crate hides.
  It doesn't look inside a projection, or inside std's adapter made a JS
  iterator (`is_iter_adapter_def` and not a compiled std type), such as
  `Zip<T::IntoIter, J>`, which its caller steps but never reads into. The
  exception is an adapter of the function's own crate, such as core's
  `map` giving `Map<I, F>`, whose struct holds what it's given as it is.
  That is asked of the function, as the walk asks of an `impl Trait`, so
  `@rust-js/std`'s own compile and each consumer answer alike (#1025).
  `is_compiled_std_type` answers for the build compiling (`compiled_for`),
  and the answer for a std function's signature mustn't depend on that.
  ADR 0411's type parameter asks the same walk with what it seeks: one
  walk, no list of containers.
- **The impl's dictionary.** `assoc_iterator_as_it_is` answers yes for
  std's `IntoIter` too, so `held_by_generic_code` makes no JS iterator of
  an impl's. `bagIntoIterator` is `{ intoIter: bagIntoIterator_intoIter,
  IntoIterIterator: () => ({ next: $rangeNext }) }`, as #2330 made every
  trait's associated iterator.
- **What a dictionary gives.** An `IntoIter` generic code got from a
  dictionary, a reference's `for<'a> &'a C: IntoIterator` (ADR 0457), is
  held as it is too (`into_iter_by_dictionary` in
  `into_iter_held_as_it_is`). It is stepped by the dictionary's
  `IntoIterIterator` entry. Only `$intoIter(t)`, of a parameter given as
  a JS iterator, still makes a JS iterator.
- **The loop stays a `for...of`.** tidy's `step_of` turned a loop stepped
  by `IIterator.next` into `for (const x of $iterator(iter, IIterator.next))`.
  It now also takes a given dictionary's entry, `C.IntoIterIterator().next`.
  That entry's call is marked `Known::Evidence` and takes no arguments; it
  makes a dictionary and does nothing else, the same fact `given_entries`
  relies on when it declares one at the top. Calling it once, where the
  loop called it each turn, is the same. The `for...of` is made only where
  no drop of the iterator follows the loop (tidy's existing rule). Where the
  iterator has a drop, the loop stays a `while` with the drop written at
  each exit.

Eleven corpus cases' JS moves, in both builds, each run as native Rust
does:
- Functions that give back or take an `IntoIter` take a dictionary:
  `generic_into_iter_values`, `option_slot_iterators`,
  `option_insert_given_drops`.
- Reference code steps through the entry, `$given(C.intoIter(c),
  C.IntoIterIterator())`: `into_iter_of_ref_param`,
  `into_iterator_of_a_reference_given_to_generic_code`,
  `from_iter_of_a_bound_reference`, `mut_kept_by_what_a_call_gives_back`.
- Earlier `while` loops over an entry become `for...of`:
  `into_iter_of_items`, `into_iterator_named_by_another_bound`,
  `mut_given_for_a_parameter_read_as_another`.
- An input `Copied<I>` no longer makes `I` held as it is, so its
  dictionaries are gone: `copied_given_to_generic_backs`.

The itertools crater check runs 196 of 415 tests as native, one more than
before: `test_std.rs::combinations`, #2328's, is taken off
`scripts/crater-decided.txt`. Corpus case `uncompiled_adapter_given_to_generic_code` gives and takes
std's adapters, `Copied`, `Filter`, `Rev`, `Zip`, `Chain`, around a
parameter and an `IntoIter`. Before this, that program was refused at a
generic `Copied`'s `next_back()`.

Tested by the associated-iterator matrix. Its `into-iter` row is off
`wrong`, and it has new rows: `into-iter-held` (an `Option`, a tuple, a
`Vec` and a `Box` given back, and an input) and `into-iter-by-a-reference`
(an impl on `&Bag` given back and stepped, and on `&mut Noisy` whose
iterator drops on a `return` inside the loop). The pair
`library_into_iter_given_back` covers a library's functions and impl
with a consumer's own `&Mine`. Mutations: `into-iter-in-a-signature-unread`,
`readable-walk-reads-into-adapters`, `impl-into-iter-a-js-iterator`,
`dictionary-into-iter-a-js-iterator`, `dictionary-entry-loop-kept`,
`readable-walk-skips-stds-own-adapters`;
`given-as-it-is-projection`, `given-as-it-is-blind-to-what-impl-trait-hides`
and `into-iter-hidden-by-a-library-unseen` retargeted to the one walk;
`impl-trait-hides-held-into-iter` merged into
`given-as-it-is-blind-to-what-impl-trait-hides`, as one walk reads both;
`into-iter-held-in-a-library-type-unseen` removed, as the walk asks no
crate of a type it holds. #2336, `&mut` of an iterator given to a
reference's bound, is a separate gap.
