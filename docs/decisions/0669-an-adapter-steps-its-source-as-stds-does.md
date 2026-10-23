# 0669. An adapter steps its source as std's does

Status: Accepted. Amends [0517](0517-scan-is-lazy-and-iterator-combinators-one-way.md),
whose amendment left an adapter over a source that isn't fused ended at
its first `None`, the guide's semantics difference 17.

Case ([0262](0262-when-rust-and-js-disagree.md)): R: results change, so
Rust's, paid for in the runtime, not in what the JS reads.

## Context

A Rust iterator may give `None` and then more: the crate's own that isn't
a `FusedIterator`, or `from_fn`'s. Std's `map`, `filter`, `enumerate`,
`skip`, `take`, `step_by`, `skip_while`, `take_while`, `zip` and `chain`'s
second half step their source each time, so they pass that on. JS's
iterator helpers, and the runtime's generators, stay ended at their first
`done`: `Flaky.map(|x| x * 10)` gave `Some(10) None None` where std gives
`Some(10) None Some(30)` (#161). `scan` and `map_while` were fixed first;
the rest was a documented difference.

The maintainer chose (2026-10-23): correctness first, and readable output
where it costs no correctness. Three ways were weighed: keep the
difference; replace every adapter with a helper, `$lazyMap(it, f)`; or the
exact adapter only where the source may restart.

## Decision

**An iterator the runtime makes of Rust's, `$iterator`'s of the crate's
own, `$fromFn`'s, a lent one and every adapter helper, is a `$Steps`:
its `next` steps on each time, and its `map`, `filter`, `take` and `drop`
are std's adapters, which pass a `None` on. The JS still reads `.map(f)`.**

- **Of a source that never gives more, each is JS's**: an exact adapter
  over a fused source steps exactly as JS's helper does, so there's no
  fused-ness to know where it's made. Arrays, ranges and std's iterators
  keep JS's own iterators and helpers.
- **`take_while` ends for good at its first item kept out**, as std's flag
  does; at a `None` of its source it ends only what's stepped now.
  `skip_while` skips till an item it keeps or a `None`. `zip` steps both
  sides on after either ends, closing them as it's closed. `chain` lets
  its first half go once it's done, and steps its second on, as std's
  `Chain` does, from either end.
- **`step_by` of a source rustc doesn't say is a `FusedIterator`** is
  `$stepBy(it, n)`, std's `nth` of each step, which a `None` stops; of one
  that's fused it's the filter of each `n`th it was, which steps alike.
- **`fuse()`, `peekable()` and `flat_map`'s outer side** stay as they were:
  std's fuse their source too.

## Consequences

- The guide's semantics difference 17 is gone.
- An adapter with items that have a destructor still closes its source
  where it reports `done` (`$closing`, ADR 0570): of a source that gives
  more after, what it then gives is stepped as std's is, and what an
  adapter held was dropped at the first end. Std drops it when the adapter
  is dropped; this shows only of a source that restarts, holding items
  with a destructor, consumed past its `None`, and is listed in the
  guide's differences.
- Held by `test/corpus/restarting_iterators.rs`, each adapter with native
  Rust, and by `scripts/mutations/runtime/steps.ts`.
