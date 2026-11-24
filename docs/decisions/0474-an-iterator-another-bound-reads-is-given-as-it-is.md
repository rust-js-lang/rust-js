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
- **A std iterator made in generic code**, `i.into_iter()`'s, is the one
  concrete code has: [0475](0475-a-generic-into-iter-is-the-iterator-concrete-code-has.md).

## Amendment: an `IntoIterator` another bound reads, and `Copy` (2026-11-25, #2540)

Case ([0262](0262-when-rust-and-js-disagree.md)): results change, so
Rust's behavior is kept.

The rule read only a type parameter bound by `Iterator`. One bound by
`IntoIterator` and another bound that reads it, `T: IntoIterator +
Clone`, was given the JS iterator its caller made of its value (ADR 0061),
which the other bound's dictionary read as the value:

- `firsts(Light::Red)` of `t.clone().into_iter()` threw `undefined is not
  a function`, its clone reading `.value` of a `$Steps`;
- `firsts(0..5)` threw too, and `shown(0..3)` of `{t:?}` printed
  `undefined..undefined`;
- `firsts(Counter { n: 0 })` printed `[Some(1), Some(2), Some(3)]` for
  native's `[Some(1), Some(1), Some(2)]`, the clone sharing its steps.

**Such a type parameter is given as it is too**, with its `IntoIterator`
dictionary, which makes its `IntoIter` as it is (ADR 0552):
`into_iter_held_as_it_is` asks the parameter's own bounds, beside its
`IntoIter`'s, by the same `reads_its_value`.

**`Copy` reads its value** too: it has no method, but its dictionary's
copy function reads the value (ADR 0289). `copied<T: Iterator +
Copy>(Light::Green)` threw `TCopy.copy is not a function`. An iterator JS
holds as no object, which generic code holds in a place of its own
(#2255), is given a copy function, `(value) => value`, which
`placed_dictionary` makes a new place of: its value is its own copy, its
place isn't.

**A library's consumer asks the same.** A library's export never called
in its library takes no `Clone` dictionary or copy function where each
value is its own (ADR 0914, amended). A consumer that gives it a value
generic code holds in a place compiles its own instance, as one crate's
analysis gives its callers one: `generic_clone_is_itself` and
`generic_copy_is_itself` in `analysis/mutation.rs` are the one answer of
both. `cloned(Light::Red)` through a library printed `[Some(0), Some(1),
Some(0)]` for native's `[Some(0), Some(0), Some(1)]`.

Corpus case `into_iterator_another_bound_reads`, crate pairs
`library_clones_an_iterator_held_in_place` and `library_iterator_shown_inside_take`.
Mutations
`into-iterator-another-bound-unread`, `copy-reads-no-value`,
`placed-copy-given-none`, `consumer-clone-of-a-place-itself` and
`consumer-copy-of-a-place-itself`.
