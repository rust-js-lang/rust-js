# 0511. The crate's traits that extend `Iterator`

Status: Accepted. Extends [0488](0488-an-extension-bound-iterator-is-an-iterator.md),
[0411](0411-an-iterator-given-as-it-is-has-its-dictionary.md) and
[0049](0049-traits-and-generics.md).

Case: N ([0262](0262-when-rust-and-js-disagree.md)): the same program, where
two representations of one value disagreed.

## Context

Probing a crate's trait that extends `Iterator`, `trait Sorted: Iterator`
with `impl<I: Iterator> Sorted for I`, found two wrong programs:

- **`through<T: Sorted>(t)` of the crate's iterator gave `[]`.** Its caller
  gave `t` as a JS iterator, as any type parameter whose bounds name
  `Iterator` is (ADR 0061), but `Sorted`'s dictionary was made for the
  impl's `I` as it is, stepped by its own `next`, which read the JS
  iterator's fields. `given_as_it_is` asked whether a parameter is an
  iterator by `Iterator` and std's extensions alone; the caller asked by
  any supertrait: two answers.
- **`&mut dyn Summed` gave `0`, or threw.** A `&mut self` method through a
  `dyn` was given the pair, `{ value, impl }`, as a box; but a trait whose
  `Self` is an iterator takes its value as it is, as generic code holds
  it (`is_generic_boxed`), and its dictionary's entries read it so.

## Decision

**A trait that extends `Iterator`, std's or the crate's, makes what it
bounds an iterator, `implies_iterator`, the one answer: a type parameter
bound by one is given as it is where its other bounds read it, as `T:
Sorted`'s methods do, and given `Iterator`'s dictionary too where it's
stepped, as `I: DoubleEndedIterator` is (ADR 0488).** And **a `dyn`'s
method is given the pair only where its dictionary takes `&mut self` in a
box, `param_is_box`'s answer**; else its value.

## Why

- **It's the same program**: `extension_trait_params` sorts, counts and
  sums through a blanket impl and concrete ones, generically and through a
  `&mut dyn`, against native Rust.
- **One answer each**: whether a bound makes an iterator, and whether a
  method's `&mut self` is a box, are each asked of one place by every path.

## Consequences

- `given_as_it_is`'s exception for the trait a function is of, whose
  `Self: Trait` is how it's called, is gone: since ADR 0492 copies a
  default into each impl, no function's own clauses name it, and its
  mutation survived every widening. So is `mir_drop`'s early return of
  what drops only as its type parameters do (ADR 0190): since ADR 0489 a
  slot no caller gives a drop for drops nothing, and its mutation survived
  the whole suite.
- The program generator's index that writes a variable no longer reads
  the vector it indexes, which rustc refused (nightly seed 12448057).
- Mutations take a crate's extension for no iterator, give it no
  `Iterator` dictionary, and give a `dyn`'s `&mut self` the pair.
