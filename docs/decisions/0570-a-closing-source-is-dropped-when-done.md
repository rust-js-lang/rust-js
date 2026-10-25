# 0570. A closing source is dropped when it's done

Status: Accepted. Amends [0465](0465-dropping-an-adapter-is-closing-it.md).

Case: B ([0262](0262-when-rust-and-js-disagree.md)): a non-fused adapter
stepped again after its `None` drops its source earlier than Rust, which
is rare and listed under Consequences.

## Context

An adapter of an iterator of the crate's that has a destructor,
`Counter { .. }.map(f).sum()`, holds its source in `$closing(it, drop)`,
which runs `drop` when the adapter is closed: `return()`, as JS calls it
when a consumer stops early, a `for` breaks, or the chain is dropped
(ADR 0465). A consumer that runs to the end, `sum()`, `collect()`,
`count()`, or a `for` that finishes, never calls `return()`: JS's iterator
helpers and `for...of` call it only on an early exit. So the source was
never dropped at all, and its `Drop` never ran, where Rust drops it as the
statement or the loop ends. A silent difference, not a refusal, found by
`test/corpus/adapter_source_drops.rs` against native Rust.

## Decision

`$closing` drops what it closes the first time its iterator reports
`done`, as well as on `return()`, once either way. A chain stepped to its
end is done with: Rust drops the temporary chain at the end of the
statement that consumed it, and the iterator a `for` took at the end of
the loop, which is where the last `next()` returns `None`. No lowering
changes: the one place that drops a closed source, `$closing`, now knows
both ways it's closed.

## Why

- **It's where Rust drops it**, as the program can observe: native Rust
  prints `drop sum` before `total 6`, and `drop for` after the last item
  and before what follows the loop.
- **One place.** The lowering already gives every such source its drop
  through `$closing`; making a consumer call `return()` at its end would
  have been a rewrite of each consumer's JS, and a second way to say
  "closed".

## Alternatives

- **Call `return()` after each consuming chain.** Each consumer's JS gets
  a `try`/`finally` or a statement after it, for a fact `$closing` can see
  itself.
- **Drop at the end of the enclosing scope**, as a drop flag does. The
  chain is a temporary with no binding to hang the flag on.

## Consequences

- A non-fused adapter stepped again after it returned `None`, by hand,
  `it.next(); it.next()` past the end, drops its source at the first
  `None`, where Rust keeps it until the adapter is dropped; the steps after
  still run on the same JS object. Only the time the `Drop` runs differs.

## What holds it

- `test/corpus/adapter_source_drops.rs`: `map(..).sum()`,
  `filter(..).collect()`, `map(..).count()` and a `for` over a `map`, each
  of a source with a printing `Drop`, against native Rust.
- Mutation `closing-not-dropped-when-done`.

## Amendment: no second close after a consumer

`v.into_iter().copied().collect()` of a source closed to drop what it
holds was also closed again after its consumer, `iter.return()`, a
second way to do what this ADR's `$closing` does as it's done: the
nightly's mutation `consumed-copied-undropped` survived, as nothing
changed but the JS. A `copied()` of such a source, consumed by value, is
closed only as it's done: `$closing(..).toArray()`.
`test/corpus/consumed_adapter_drops.rs` holds it against native Rust.

## Amendment: done however it's stepped, and by what ends early

An iterator of the crate's with a destructor was never dropped through
much of std's adapters (#225): of 2,100 generated programs, 226 never
ran its `Drop`, and 30 of a `fuse()` of it stepped from its back threw.
`$closing` knew it was done by its `next` alone. Now:

- **Done however it's stepped**: by its `next`, its `back`, its `nth` or
  its `nth_back`, as a `step_by` steps the crate's own `nth` and a back
  form steps its back. It passes `$knowing` on to what it closes, so a
  compiled `fuse()` closed to drop it is given its back and length.
- **What ends early is done with what it steps**, as `$Steps`' `take` at
  its limit already was: a `skip` or a `take` ended from its back, a
  `take`'s `try_fold` asked again at its limit (a `filter`'s step), and a
  `skip`'s or a `take`'s `rfold`, which consumes it.
- **What's taken is closed to drop it**: a `filter` stepped from its back
  closes its source; `rev()` and an `enumerate()` made knowing its back
  close what they're given, by its own type, so not through a `&mut`.
- **What a `&mut` lends is stepped as it**: `user_iterator_knowing` sees
  through `$lent(..)`, as through `$closing(..)` (ADR 0465), and gives a
  `&mut` of the crate's iterator its own methods as std's `&mut I`
  passes them on, not its folds, which take and drop it:
  `(&mut it).map(f).rev().sum()` was 0, not 90, a wrong value.

Of the 2,100, 216 more agree; 57 still differ. Of those, 25 are a
`fuse()`'s own `rfold` and `nth_back` (#224). Nine are a chain stepped to
its end by a consumer that borrows it, `nth` or `find` say, whose source
is dropped as it's done, not at the statement's end. That's #206's
question of when done means dropped. Two drops of a `filter(..).fuse()`
stepped from its back no longer reach the crate's `rfold`, as `rev()`
closing a compiled `fuse()` hides it from "a `Fuse` stepped to its end is
its source" (ADR 0561). #224's fix covers them too.
`test/corpus/own_iterator_dropped_through_adapters.rs` holds the rest,
with 17 mutations; `closing` leaves what's closed already as it is, so an
owning source `rev()` is given isn't closed twice. The readability
baseline's `while (true)` share grows by 0.1 in 10,000 lines: the case's
`rfold`, a `while let`.
