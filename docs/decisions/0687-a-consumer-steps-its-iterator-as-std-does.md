# 0687. A consumer steps its iterator as std's does

Status: Accepted. Amends [0057](0057-ordering.md),
whose `$max(it.toArray())` listed the items first, and
[0669](0669-an-adapter-steps-its-source-as-stds-does.md), whose `$Steps`
now has std's `fold` and `nth` too.

Case ([0262](0262-when-rust-and-js-disagree.md)): R: results change, so
Rust's, paid for in the runtime; the JS reads as it did, or better.

## Context

Two ways the JS stepped an iterator otherwise than std does, each seen in
what a program prints:

- **A consumer's closure ran after every item was stepped** (#179).
  `max_by_key`, `max_by`, `min_by_key`, `min_by`, `max`, `min`,
  `partition` and `unzip` were given `it.toArray()`: every `next`, and
  every closure of an adapter before them, ran before the first key,
  compare or predicate. Std steps one item and gives it on before the
  next, so `next, key 1, next, key 2` came out `next, next, key 1, key 2`.
- **The crate's iterator's own `fold` and `nth` weren't called** (#162).
  Std's `sum`, `count`, `last`, `for_each`, `fold`, `max` (after its
  first item), `partition`, `unzip`, `collect` into a `String`, a
  `HashMap`, a `HashSet` or a `LinkedList`, and `extend` of those, route
  through `fold`; so do the adapters whose `fold` is their source's, `map`,
  `filter`, `enumerate`, `inspect`, `skip`, `chain`, `flat_map`. `skip`'s
  first step, `step_by`'s steps and `nth` through a `&mut` call `nth`. The
  JS stepped `next` in each, so an impl that writes its own, to be faster
  or to do more, wasn't called. It was the guide's semantics difference 17.

The maintainer chose (2026-10-23) to honor them: correctness first.

## Decision

**Each consumer steps its iterator item by item, as std's does, and
through the crate's own `fold` and `nth` wherever std's does.**

- **The helpers take the iterator, not its array**: `$maxByKey(it, key,
  compare)`, `$partition(it, keep)`, `$unzip(it)`, `$max(it)`, where it
  was `$maxByKey(it.toArray(), ..)`. Each steps it by std's own
  consumer's steps: `max` and the rest by `reduce`, its first item by
  `next`, then its `fold` of the rest; `partition` and `unzip` by
  `for_each`, its `fold`. `$max` is `$maxBy(items, $cmp)`, std's
  `max_by(Ord::cmp)`. Of an array, made where nothing's closure can tell
  the order (ADR 0364), each is still given the array, `$max(xs)`.
- **`$iterator` is given the crate's own `fold` and `nth`** where its impl
  writes them: `$iterator(up(), upIterator_next, false, undefined,
  undefined, undefined, upIterator_fold, upIterator_nth)`. Its `reduce`
  and `forEach` call its `fold`, and its `drop` (`skip`) its `nth`. Its
  `map`, `filter` and `flatMap`, and `$lazyChain`, fold by their
  source's; `take` doesn't, as std's `Take` doesn't. `$lent`, a `&mut`,
  gives `nth` to its iterator's, as std's `&mut I` does, and folds by
  `next`. A `$Steps` is still a JS iterator: in JS, `reduce(f, init)` is
  the one way to fold any iterator, which is a `$Steps`'s `fold` or a JS
  iterator's steps.
- **What JS steps by its own protocol is given the items `fold` gives**,
  where std's routes through `fold` and that reaches the crate's own:
  `$folded(it)`, of `last()` and of `collect` into a `String`, a hash map
  or set, a list or `()`. Whether it reaches it is
  `Recognition::folds_its_own`, of the iterator's type: the crate's
  iterator whose impl writes `fold`, and each adapter whose `fold` is its
  source's over one. A `Vec`, a deque, a heap and a B-tree collect by
  `next`, as std's do, so `toArray()` and `new Map(it)` stay as they were.
  `extend` asks the same of what it extends (`collects_by_fold`): a hash
  map's or set's, a B-tree's, a string's or a list's folds, a `Vec`'s, a
  deque's or a heap's steps.
- **A map or a set is extended by `iterable_items`**, the one place an
  iterable becomes JS items. Its own `items_of`, which made only a range
  JS items, gave `$extendMap(s, up(), true)` the crate's iterator's struct,
  which JS can't step: a `TypeError` where std extends it. `items_of`, and
  the `$charRange` and `$bigRange` it alone made, are gone; a `char`
  range extending a set reads as every other `char` range made where it's
  stepped does, `$iterator({ start, end }, $rangeInclusiveNext)`.
- **Behind a `dyn Iterator`, `fold` is std's**: `Box<I>` and `&mut I`
  forward `nth` only, so the `dyn` is made without the crate's `fold`
  (`behind_a_dyn`).
- **An iterator a helper steps till it ends** is given no `size_hint`
  nothing asks, as one `toArray()` steps wasn't
  (`HELPERS_STEPPING_ALL`, the one list of those helpers).

## Consequences

- The guide's semantics difference 17 is narrowed to what's left:
  generic code given the crate's iterator as it is, by its dictionary
  (ADR 0411), a `Peekable` of it among them, steps its `next`.
- In a library (ADR 0100), the crate's own `fold` is called through the
  runtime without the drops of its accumulator and closure, which the
  runtime can't name: of an accumulator with a destructor that the impl's
  `fold` drops, or that a panic in it unwinds, nothing is dropped. A
  `fold` that takes other evidence, a bound's dictionary, is refused.
- The JS reads better: `$max(evens(v).map(f))` for `$max(evens(v)
  .map(f).toArray())`, `$partition(v, keep)` for `$partition(v.slice(),
  keep)`.
- Held by `test/corpus/consumers_step_each.rs`,
  `test/corpus/own_fold_and_nth.rs` and `restarting_iterators.rs`'s
  `flat_map`, each with native Rust, and by the mutations of each file
  changed.

## Amended by

[ADR 0690](0690-an-iterator-given-as-it-is-folds-by-its-own-fold.md): an
iterator given as it is, by its dictionary, `Peekable`'s and `Fuse`'s
sources among them, folds by its own `fold` too, and an iterator that
fold takes is dropped once.

[ADR 0711](0711-what-stops-early-steps-by-try-fold.md): what stops early,
`any`, `all`, `find`, `position`, `try_fold` and `eq`, steps by std's
`try_fold`, which `skip` makes its source's `nth(n - 1)`.
