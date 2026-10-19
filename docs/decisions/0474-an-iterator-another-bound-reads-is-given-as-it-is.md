# 0474. An iterator another bound reads is given as it is

Status: Accepted. Extends [0411](0411-an-iterator-given-as-it-is-has-its-dictionary.md)
and [0061](0061-generic-iterators.md).

Case: C ([0262](0262-when-rust-and-js-disagree.md)): the same values, as
the representation each reads.

## Context

A generic iterator given only itself, `it: I` of `I: Iterator`, is a JS
iterator (ADR 0061): the crate's own given for it is `$iterator(value,
next)`. But another bound reads it as its own type: `I: Clone`'s
`clone(&it)`, `I: Debug`'s `fmt(&it)`, each given the crate's own impl,
which read the `$iterator` as the crate's value. `dup(Count(0))` printed
`Count(undefined)` and cloned nothing.

## Decision

**A type parameter bound by `Iterator` is given as it is (ADR 0411) where
another bound has methods that read it**, `Clone`, `Debug`, `PartialEq`:
its value, the crate's own or std's, and its `Iterator` dictionary, `{ next
}`, which steps it.

- **Not a marker**, `Send`, nor the trait the function is of, whose `Self:
  Trait` is how it's called, an extension trait's of any iterator.
- **The one answer**, `given_as_it_is`, of the callee's evidence and the
  caller's argument alike.

```js
function dup(it, IIterator, IClone, IDebug) {
  IIterator.next(it);
  console.log(`${IDebug.fmt(it)}`);
  return [$iterator(IClone.clone(it), IIterator.next, true).toArray(), $iterator(it, IIterator.next, true).toArray()];
}
```

## Why

- **It's the same value**: `iterator_read_as_itself` clones and shows the
  crate's iterator, a `Vec`'s and a slice's, each stepped first, with native
  Rust.

## Consequences

- **What was wrong is right**: a crate's iterator given to a generic
  function that clones or shows it.
- **Still to close**: a std iterator made in generic code, `i.into_iter()`'s
  `Iterator.from(v)`, and read in concrete code as the `$iter` it would be
  there, its `len()` or clone.
