# 0487. An iterator given as it is has its extensions' dictionaries

Status: Accepted. Extends [0411](0411-an-iterator-given-as-it-is-has-its-dictionary.md),
[0429](0429-a-double-ended-iterator-knows-its-back.md) and
[0456](0456-a-generic-exact-size-iterator.md).

Case: C ([0262](0262-when-rust-and-js-disagree.md)): each step, and each
count, is the one Rust's would take, of the value Rust's code has.

## Context

A type parameter given as it is, one a type holds or another bound reads,
`I: Iterator + Clone`, is stepped by its `Iterator` dictionary (ADR 0411).
Its `DoubleEndedIterator` and `ExactSizeIterator` bounds weren't given one:
`next_back()`, `len()` and `rev()` of it were refused, and so was giving it
on to generic code that asks its back or its length. itertools'
`circular_tuple_windows` asks `iter.len()` of an `I: Clone +
ExactSizeIterator`. And one given to an adapter, `(0..2).chain(iter)`, was
spread as a JS iterator, which a range or the crate's own iterator isn't.

## Decision

**An iterator given as it is is given its extensions' dictionaries as it
is its `Iterator`'s, where it's stepped by them: `DoubleEndedIterator`'s
`{ next_back }` and `ExactSizeIterator`'s `{ len }`.** Its `next_back(&mut
it)` is `IDoubleEndedIterator.next_back(it)`, its `len()`
`IExactSizeIterator.len(it)`, and `rev()` a JS iterator stepped by
`next_back`.

```js
function twice(iter, IIterator, IClone, IExactSizeIterator) {
  const len = IExactSizeIterator.len(iter);
  ...
}
twice({ start: 0, end: 3 }, { next: $rangeNext }, { clone: .. }, { len: $rangeLen });
```

- **A dictionary's entry steps the type as it is**: a range's
  `$rangeNextBack` and `$rangeLen`, the crate's own iterator its impls',
  its `len()` std's, `$exactLen` of its `size_hint()`; a `$iter`'s
  `$nextBack` and its own `len()`.
- **Given as a JS iterator, it's one made of them**: to `last<K:
  DoubleEndedIterator>(k)`, `$iterator(it, IIterator.next, true,
  IDoubleEndedIterator.next_back)`; to an adapter, `chain(it)`,
  `$iterator(it, IIterator.next)`. One helper, `given_js_iterator`, makes
  it for each.
- **Only the dictionaries it's stepped by**: the analysis
  (`stepped_params`) now finds which trait steps each parameter, an
  extension's method, or an `Iterator` method and the extensions its
  clauses ask of `Self`, `rev()`'s `DoubleEndedIterator`; one given on as a
  JS iterator is stepped by `Iterator` and by each extension its callee's
  parameter is bound by. `back(ends)`, which only steps from the back, is
  given `IDoubleEndedIterator` alone.

## Why

- **It's std's iterator**: `given_iterator_extensions` asks the length
  and the back of a vector's iterator, a stepped slice's, a range and the
  crate's own iterator given as they are, reverses them, chains them, and
  gives them on to generic code that asks either, with native Rust; and
  `next_back_generic`, the refusal it was, runs.

## Consequences

- **What compiles grows**: `next_back()`, `len()` and `rev()` of an
  iterator given as it is, and giving it where its back or length is
  asked.
- **`rev()` takes its `Iterator` dictionary too**, as an `Iterator`
  method, though it steps only from the back.
- **One bound only through a supertrait, `I: DoubleEndedIterator` a type
  holds, is still a JS iterator** in its impl, which its caller's value
  isn't; that's the next gap. (Amended: it's given as it is, its extension
  bound an `Iterator` bound too, ADR 0488.)
