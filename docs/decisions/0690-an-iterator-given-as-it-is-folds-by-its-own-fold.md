# 0690. An iterator given as it is folds by its own `fold`, and drops once

Status: Accepted. Amends [0687](0687-a-consumer-steps-its-iterator-as-std-does.md),
whose consumers called the crate's own `fold` of an iterator made a JS one
but not of one given as it is, and
[0411](0411-an-iterator-given-as-it-is-has-its-dictionary.md), whose
dictionary now carries that `fold` and `nth`.

Case ([0262](0262-when-rust-and-js-disagree.md)): R: results change, so
Rust's, paid for in the runtime and, in generic code that drops, one test.

## Context

ADR 0687 left one place where the crate's own `fold` wasn't called:
generic code given the iterator as it is, by its `Iterator` dictionary
(ADR 0411). Std's own bodies compiled into `@rust-js/std` are such code
(ADR 0561), `Peekable`'s and `Fuse`'s among them, and so is a generic
struct holding an `I: Iterator`. The dictionary was `{ next }`, so
`Holder { it }.total()`, `peekable().count()` and `fuse().sum()` stepped
`next`.

Honouring it showed three drops wrong:

- **Twice:** a `fold` takes `self` and drops it at its end. The lowering
  also dropped a consumed iterator after its consumer, where std's default
  `fold` would have. With the crate's own `fold`, that was a second drop,
  as ADR 0687 already had of `Own.sum()`.
- **Never:** a `map` of a `peekable()` of an iterator with a destructor
  never dropped it. `$closing` closed the crate's own iterator, but not a
  compiled std struct holding one. This is older than #162.
- **And a lost name:** the `$closing` drop of such a struct took the name
  `fuse` from the binding it closed.

## Decision

**A crate iterator's dictionary carries its own `fold` and `nth`. What
steps a value by its dictionary folds by them, and the iterator is dropped
once: by its own `fold`, which takes it, or after its consumer.**

- **The dictionary** is `{ next, fold, nth }` where the impl writes them,
  as it carries `size_hint` (ADR 0170).
- **Generic code steps it by `$given(it, IIterator)`**, which reads the
  dictionary's `next`, `fold`, `nth` and `size_hint`, where it read
  `$iterator(it, IIterator.next)`.
- **`Peekable`'s and `Fuse`'s folds are std's**: `$peekableFold` folds what
  it peeked at, then its source by its dictionary (`$foldBy`), and
  `$fuseFold` folds its source while it has one. `$stdIterator` is a
  `$Steps`, so its `reduce` and `forEach` call them.
- **Dropped once:**
  - `dropped_after` drops nothing after a consumer that takes its items by
    `fold` (`consumes_by_fold`, its names `Recognition::folds_what_it_consumes`'s)
    where the fold reaches the crate's own (`folds_its_own`).
  - In generic code it can't know, so the drop asks the dictionary:
    `if (IIterator.fold === undefined) { dropI?.(iter); }`.
  - `$closing` folds by its iterator's fold and closes after, except where
    that was the crate's own (`ownFold`), which dropped it.
  - An adapter holding a closure with a destructor over such an iterator
    is refused: its `fold` drops only itself.
- **A compiled std struct holding an iterator with a destructor is closed
  by `$closing`**, by its fields' drops, its parameter `it`, as the crate's
  iterator's `it.of` is.

## Consequences

- The guide's semantics difference 17 is narrowed to the library's: the
  crate's own `fold` isn't given its accumulator's drops (ADR 0687).
- A generic function given, by value, an iterator with a destructor that
  it steps by `nth` is refused (#180), as it was before.
- Held by `test/corpus/own_fold_through_adapters.rs`, against native Rust,
  and by the mutations of each file changed.

## Amended by

[ADR 0711](0711-what-stops-early-steps-by-try-fold.md): `$closing` and
`$lent` pass on their iterator's `try_fold` and `nth` as they do its
`fold`.
