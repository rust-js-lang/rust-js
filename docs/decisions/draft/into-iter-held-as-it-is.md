# An `IntoIterator` whose iterator is held as it is is given as it is

Status: Draft. Would amend [0061](../0061-generic-iterators.md) and extend
[0411](../0411-an-iterator-given-as-it-is-has-its-dictionary.md) and
[0457](../0457-an-into-iterator-bound-on-a-reference.md).

Case: C ([0262](../0262-when-rust-and-js-disagree.md)): a generic
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

- **One answer, `into_iter_held_as_it_is(id, X)`**, which
  `into_iter_by_dictionary`, `given_as_it_is`, the analysis and the
  caller's evidence all ask. Its value-reading rule is `given_as_it_is`'s
  own, shared and not copied.
- **The caller's dictionary**: of a range, `{ into_iter: (range) => range }`;
  of a `Vec`, `{ into_iter: (items) => $iter(items) }`, std's iterator over
  an array as concrete code has it; of the crate's collection, its impl's
  `into_iter`; of one given as it is, the caller's own, passed on.
- The caller gives `X` as it is, not made a JS iterator
  (`given_as_iterator` asks it).
- Everything else is ADR 0061's, unchanged: an `IntoIterator` whose
  iterator nothing holds as it is is still given a JS iterator.

## What would hold it

- `into_iter_held_as_it_is`: `twice` of a range, a `Vec` and a crate's
  collection, and a `multi_cartesian_product`-shaped one through
  `H::Item`, against native Rust.
- itertools' probe: `multi_cartesian` the same as native.
- Mutations: the rule answering no, the dictionary's `into_iter` for each
  kind, and the caller converting what it gives.

## Open

- A library's `X: IntoIterator` bound read by its consumer: the manifest
  says it, as `stepped` does (ADR 0548), or the consumer reads the clauses.
- A second gap that the same probe shape found:
  `std::mem::replace(&mut self.orig, ..)` of a field holding a type
  parameter is refused, "this `&mut` given to `mem::swap` and the like".
