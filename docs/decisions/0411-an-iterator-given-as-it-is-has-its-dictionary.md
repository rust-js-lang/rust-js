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
