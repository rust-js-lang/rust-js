# 0525. A cloned `map` or `filter` is made again

Status: Accepted. Extends [0522](0522-a-closure-cloned-apart-is-made-again.md)
and [0181](0181-iterator-clones.md); accepts part of the
[iterators draft](draft/cloned-and-buffering-iterators.md).
Amended by [0984](0984-a-map-or-filter-cloned-apart-keeps-its-back.md): one
stepped from its back is made knowing it.

Case: C ([0262](0262-when-rust-and-js-disagree.md)): a clone of an iterator
is where the iterator is, with a clone of its closure, as Rust's `Clone` of
a `Map` is; nothing reruns.

## Context

A lazy adapter is a JS iterator helper, `it.map(f)`, whose state hides in
the engine: a clone of one was refused ("cloning
`std::iter::Map<std::ops::Range<i32>, {closure}>`"). A `map` or a `filter`
holds nothing but its source and its function, though: a JS helper's
position is its source's. A clone is the helper made again of a clone of
each, as a closure cloned apart is (ADR 0522).

## Decision

**A `map` or a `filter` the crate clones is made by a function of its
source and its function, `makeMap`, and carries its clone,
`$cloning(it.map(f), () => makeMap(clone(it), clone(f)))`; `$clone(it)`
makes it again.**

```js
const makeMap = (it, f) =>
  $cloning(
    $iterator(it, $rangeNext).map((item) => f(item)),
    () => makeMap({ start: it.start, end: it.end }, $clone(f)),
  );
const it = makeMap({ start: 1, end: 4 }, counter);
const copy = $clone(it);
```

- **One owner of what's cloned**: `analysis::clones`, the types each call's
  `Clone` bounds are given, revealed through `impl Clone`, and what each
  clone clones in turn, its `Clone` impl's bounds, to a fixed point:
  `Map<I, F>: Clone` clones `I` and `F`. Closures read their part of it
  (ADR 0522).
- **One question, `iterator_cloned_apart`**: a `Map` or `Filter` of the
  cloned types, where a generic function's `Map<I, {closure}>` stands for
  each its callers clone; what's made and what's cloned both ask it.
- **Its source is cloned as it is**: an array's iterator's, a range's,
  another made again; a `copied()` is the iterator it copies (ADR 0480).
- **A closure that writes nothing it took is shared**: `clone_shares` asks
  what its captures do, not its kind, which a `map`'s bound makes `FnMut`.

Still refused, in [the limitations](../guide/limitations.md): a clone of one that
keeps a count of its own, an `enumerate`'s, a `take`'s or a `skip`'s, whose
JS helper hides it; one of items with a destructor, or lent; and one
generic code makes where its source isn't bound by `Clone`, which it
couldn't clone.

## Why

`lazy_iterator_clone` clones a `map` of a closure that counts its calls,
partly stepped, a `filter` of a `map`, one through a `T: Clone`, one a
function gives as an `impl Iterator + Clone`, and one a derived `Clone`'s
field holds, against native Rust.

## Consequences

- `docs/guide/limitations.md` loses "A clone of a lazy iterator", and gains its
  narrower remains.
- Mutations stop following impl bounds, revealing an `impl Clone`, or
  matching generic code's closure; share a source or a closure; and make
  every `map` cloned apart where a type parameter is cloned.

## Amendment: an `enumerate`, a `take` and a `skip`

An adapter that keeps a count of its own isn't made again of its source
alone: its count is state a `map`'s function isn't. A clone of one is
ADR 0542's, an explicit adapter that keeps its count where its clone
reads it (that ADR's amendment). Only a `Map` and a `Filter`, of a source
and a function alone, are made again.

## Amended (#181)

A `Chain` and a `Zip` are cloned apart too: adapters made of two parts
alone, their two sources, as a `Map` and a `Filter` are of their source
and function (`Recognition::cloned_parts`, which says which it is,
`Parted`). `makeChain(a, b)` is `$lazyChain(a, b)`, carrying its clone,
`makeChain` of a clone of each, where it is.

The other side of a `chain(..)` or a `zip(..)` is an `IntoIterator`.
Given as an iterator, it is as the receiver is: its JS value says where
it is. Given as a `Vec`, an array or a slice, it's made the array's
iterator before the maker (`$iter`), and cloned as the iterator the
adapter keeps, so a clone after stepping goes on from there. Another,
whose JS value can't say where it is, `once(x)`'s array, is refused.

And `cycle()` of an adapter cloned apart is `$cycleOf(it, $clone, $next)`:
`cycle` requires `Self: Clone`, so the crate's clones already hold it,
and each round steps a fresh clone of the original, its closures run
again, as std's `Cycle` does.

Held by `test/corpus/cloned_chains_and_cycles.rs`, against native Rust, and
by the mutations `chain-not-cloned-apart`, `zip-not-cloned-apart`,
`zip-side-array-uncloned` and `cycle-of-cloned-apart-refused`.


## Amended (#384): a `copied()` of the crate's own, or of what generic code is given

"A `copied()` is the iterator it copies" held of std's iterators, whose JS
value is their JS iterator, and not of the crate's own, which is stepped
as `$iterator(value, next)`, nor of what generic code is given, which
`$given` makes a JS iterator of where its dictionary is the crate's. A
clone of either was a clone of that JS iterator as the value it steps:
`{ ...copied }`, which threw, or `IClone.clone(copied)`, an empty clone,
`(5, 0)` where Rust gives `(5, 3)`.

Case: C ([0262](0262-when-rust-and-js-disagree.md)): the clone is where
the iterator is, apart from it, as Rust's `Clone` of a `Copied` is.

- **Of the crate's own, a clone is a JS iterator of a clone of its
  value**, stepped as it is: `copied.with({ ...copied.of })`, `$iterator`'s
  `with`.
- **Of what generic code is given, `$givenClone(copied, IClone.clone)`**:
  where `$given` made a JS iterator of the value, which it marks `given`,
  one of the value's clone, marked too, so a clone of the clone is one;
  else the value's clone, as before.
- **Refused**: one closed to drop what it steps, the crate's own with a
  destructor or one generic code is given its drop of, `$closing(it,
  drop)`; and a copy of items something changes in place, `source.map((item)
  => ({ ...item }))`, a JS iterator of JS's (`is_its_source`). Each threw
  before; each is in [the limitations](../guide/limitations.md).

Held by `test/corpus/copied_clone_of_its_own.rs`, against native Rust, and
by the mutations `copied-own-clone-spread`, `copied-given-clone-by-dictionary`,
`given-clone-of-the-wrapper`, `given-clone-unmarked`,
`copied-dropped-clone-allowed` and `copied-copying-clone-allowed`.
