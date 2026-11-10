# 0411. An iterator given as it is is stepped by its dictionary

Status: Accepted. Amends [0061](0061-generic-iterators.md), extends
[0049](0049-traits-and-generics.md).

Case: C ([0262](0262-when-rust-and-js-disagree.md)): each step is the one
Rust's would take, of the value Rust's code has, a range's start moved in
place; where it can't be found, D: rust-js's error.

## Context

ADR 0061 has generic code take an iterator as a JS iterator: a range or one
of the crate's own is made one where it's given, `$lazyRange(0, 4)`,
`$iterator(it, countdownIterator_next)`. That's right of a parameter given
only itself, `total<I: Iterator>(i: I)`, which nothing gives back. But an
adapter, `struct Doubled<I> { inner: I }` whose `next` calls
`self.inner.next()`, holds its iterator: `Doubled { inner: 0..4 }` holds a
range, which its caller can still read, `d.inner.start`, and a function
that gives back what it's given, `doubled<I>(inner: I) -> Doubled<I>`,
would give back a JS iterator its caller reads as a range. rust-js refused
a `&mut` to a generic iterator a field holds.

## Decision

**A type parameter bound by `Iterator` that an impl's type holds, or that
a function's signature has in anything but itself, `Wrap<I>` or its
result, is given as it is: a range a range, the crate's own iterator
itself. Where it's stepped, it's given its `Iterator` dictionary, `{ next
}`, as a trait's bound is (ADR 0049): `IIterator.next(it)`, and of any
other method, a JS iterator of it, `$iterator(it, IIterator.next)`. One
given only itself is still a JS iterator (ADR 0061).**

```js
function doubledIterator_next(doubled, IIterator) {
  const x = IIterator.next(doubled.inner);
  return x != null ? Math.imul(x, 2) : undefined;
}
const w = $iterator({ inner: { start: 0, end: 4 } }, (iterator) =>
  doubledIterator_next(iterator, { next: $rangeNext }),
).toArray();
```

- **A dictionary's `next`** is what steps the type as it is: a range's
  `$rangeNext`, the crate's own iterator's impl's `next`, a JS iterator's
  `$next`, or `$nextSome` of items that may look like `None`, each giving
  the `Option` generic code reads (ADR 0051).
- **One answer** says which parameters are given as they are,
  `given_as_it_is`: the callee's evidence asks it, and its caller's
  argument, which it then doesn't make a JS iterator.
- **Only where it's stepped**: an analysis (`stepped_params`) finds each
  function that steps the parameter, directly, through the crate's
  iterator holding it, or through a function it gives it to; only those
  take the dictionary, so `doubled(inner)` takes none. One it missed, that
  lowering asks for, is rust-js's error, never a JS iterator's `next` of a
  range.
- **`by_ref()` and `into_iter()`** of one are itself; a `for` over one is
  `for (const x of $iterator(it, IIterator.next))`.

## Why

- **It's the program Rust runs**: the `iterator_adapters` corpus case
  steps an adapter of a vector's iterator, a range, the crate's own
  iterator and another adapter, reads the range it holds after a step,
  gives one back and steps it after, sums through a generic function, and
  steps adapters of items that may be `None`, each with native Rust.
- **The refusal it replaces is gone**: a `&mut` to a generic iterator a
  field holds is either given as it is, with its dictionary, or a JS
  iterator, which `$next` steps.
- One more of rustc's tests passes, `iter-map-fold-type-length.rs`, and
  every in-scope test of `iterators/` and `for-loop-while/` still does.

## Consequences

- A trait's method's own iterator, `&mut self` of a blanket `impl<I:
  Iterator> Trait for I`, is still refused. (Amended: it isn't. Its
  dictionary's entry takes the box generic code gives a `&mut Self` and
  gives the method what's in it, the iterator, which its `Iterator`
  dictionary steps, as an object's impl's entry does; and a copied
  default's `&mut self` given to the impl's method is what's in its box,
  as a type parameter that's a JS iterator isn't boxed (ADR 0099). The
  `blanket_iterator_trait` corpus case steps a range in place, the crate's
  own iterator, a slice's and an adapter of the trait's through it, with
  native Rust; rustc's `streaming_iterator.rs` passes.)
- A loop over a generic adapter's iterator of `Clone` items reads
  `$iterator(IClone.clone(items), IIterator.next, true)` where it read
  `IClone.clone(items)`: right of a range too, which that wasn't.

## Amended by

[ADR 0690](0690-an-iterator-given-as-it-is-folds-by-its-own-fold.md): the
dictionary carries the crate's own `fold` and `nth`, and a value given
as it is is stepped by `$given(it, IIterator)`, which reads them.

## Amendment: a `&mut I` and another `I` are given as they are (#537)

A function whose signature has a `&mut I` and another `I` or `&mut I`,
`replace_in<I: Iterator>(it: &mut I, next: I)` or `swap_in(a: &mut I, b:
&mut I)`, may write one where its caller holds the other, so `I` is given
as it is, as one it gives back is: its caller's own value, the same
representation as the place it's written to, and `*it = next` is
`$assign(it, next)` (ADR 0481), where of a JS iterator it was refused.
One with a single `&mut I`, or `I`s alone, stays a JS iterator (ADR
0061). See [ADR 0871's amendment](0871-rev-is-stds.md#amendment-what-holds-a-value-doesnt-step-it-so-one-type-is-one-representation-537).

## Amendment: an iterator another bound names is given as it is (#966, #969)

`fn backwards<I: DoubleEndedIterator>(it: impl Is<I>)`, `fn first<I:
Iterator, P: Into<I>>(given: P)` and `fn made<I: Iterator, F: FnOnce() ->
I>(make: F)` give their bodies an `I` that no parameter of type `I` holds:
`it.is()`, `given.into()` and `make()` give what the caller gave as `P`.
`I` was a JS iterator its caller made (ADR 0061), so it was stepped as one,
but no caller made it: `P`'s value went as `P`'s representation, which knows
no back and no length, and whose drop reads another's. Rust's answer
changed (`TypeError`, a lost drop), so Rust's behavior is kept (ADR 0262's
third case).

`given_as_it_is` now answers yes of an iterator parameter that another
bound names in its arguments or its projection's term, as one held in the
signature is, and yes of the parameter whose own bound names such an
iterator, `impl Is<I>`'s, `P`'s, `F`'s: the one value passes between them,
so it has one representation, the caller's own value and its dictionaries.
The `iterator_named_by_another_bound` corpus case runs each with native
Rust, from the back, by its length, and dropped; the metamorphic net's
`impl-ref` and `impl-owned` rewrites of the corpus, which make every
generic parameter so, run as native.

## Amendment: one answer of a type parameter another bound names (#1109)

`given_as_it_is` gave a type parameter bound as an iterator that another
bound names, `fn backwards<I: DoubleEndedIterator>(it: impl Is<I>)`'s `I`,
as it is by its last test, `named` (d6ea7894a, #966,
`iterator_named_by_another_bound.rs`). c2ab4dc29 (#969) answered the same
in `into_iter_held_as_it_is`, of a parameter bound by `Iterator` or
`IntoIterator` that another bound names, which `given_as_it_is` asks
first, of every parameter, and an iterator's is one. The last test never
decided anything: a probe at it logged none in the whole net, its mutation
`given-as-it-is-not-named` survived, and both `named_by_another_bound`
cases pass without it. With #969's test taken out too
(`into-iter-named-not-held`), both fail, the iterator's case as well as
the `IntoIterator`'s, so #969's is the one answer, and `named` is removed
with its mutation. `iterator-read-as-js` is retargeted at what's left.

## Amendment: one given to a std function rust-js lowers itself is stepped (#1147)

An impl takes a type parameter's `Iterator` dictionary where one of its
bodies steps it (`iterator_steps::stepped_params`): a method of
`Iterator`'s, or a function whose manifest says it steps it. A `Debug`
builder's `entries(items)` is neither: std's, lowered by the display
writer, with no manifest. So `impl<F, I> Debug for Listed<F> where F:
Fn() -> I, I: Iterator`, whose `I` is given as it is (another bound
names it, #969), took no `IIterator`, and its `entries((self.0)())`
stepped the crate's own iterator the caller's closure made as a JS one,
`[[object Iterator]]` of `It(vec![1, 2].into_iter())` where native Rust
shows `[1, 2]`, or a TypeError.

**A caller's type parameter given as it is, given to a std function or a
std trait's method with no manifest, as one of its own type parameters
that it bounds by `IntoIterator` or `Iterator`, is stepped by
`Iterator`**, as std's body steps it: its dictionary is given, and
`entries` and `extend` step it by `$given`. Not one its impl's type
holds, `Peekable<I>`'s for `next_if`, which its own lowering steps. The
consumer net's generic callers (ADR 1090) found `Extend::extend`'s too,
of each collection and `String`, where the first rule took std's
functions alone. Mutations `std-iterable-param-unstepped` and
`std-iterable-of-trait-method-unstepped`;
`test/corpus/debug_entries_drop_as_std.rs` and
`test/corpus/generic_iterables_given_to_std.rs`.

## Amendment: what another bound names goes as it is, never wrapped (#1109)

`names_given_as_it_is` (#966, #969) gives a type parameter as it is whose
own bound names an iterator given as it is, or an `IntoIterator` held as
it is: `impl Is<I>`'s, `P: Into<I>`'s. Since `into_iter_held_as_it_is`
answers for the named parameter itself, no output changed with either of
its two tests taken away, and both mutations survived (#1109). What they
still decide is the form of what's given for it: one not given as it is
is wrapped in its slice's steps where its items' drop may panic,
`$slicing($iter(v), drop)`, though `I`'s own dictionary, a `Vec`'s
iterator's, steps it as one slice already, and runs the same. So it's
held by the snapshots: `iterator_named_by_another_bound.rs` and
`into_iterator_named_by_another_bound.rs` each give a `Vec`'s iterator of
items whose drop may panic for an `impl Is<I>`, against native Rust in
both builds, and `naming-parameter-not-as-it-is` and
`naming-parameter-of-into-iter-boxed` check their JS.

## Amendment: what an `impl Trait` hides holds it too (2026-11-10, #1230)

`given_as_it_is` read a signature's types for a value of the parameter,
`has`, and skipped every alias in them, an opaque type's too: `fn
hidden<I: Iterator>(it: I) -> impl Iterator<Item = I::Item> { it }` gave
`I` back hidden, so its caller made the crate's own `Count` a JS iterator,
`$iterator(count, next)`, for it, then revealed the call's result as the
`Count` it gave (#1142) and made a JS iterator of that again, which
yielded nothing (found by #1204's `opaque` rewrite). What an opaque type
of the crate's hides is read as the type it is, `type_of(..)` of its
arguments, as `into_iter_held_as_it_is` reads one (ADR 0920), each once
(`holds_value_of`): a parameter it holds, itself, the crate's struct of it
or a `map` of it, is given as it is, as one a signature names is. The
user's decision, option A of #1230, over leaving the opaque unrevealed
at the caller; it changed no corpus case's JS but the new one's.
Results changed ([0262](0262-when-rust-and-js-disagree.md)), so Rust's are
kept. Corpus case `own_iterator_given_on_behind_impl_trait`, both
builds; mutation `given-as-it-is-blind-to-what-impl-trait-hides`.
