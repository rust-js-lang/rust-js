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
