# 0517. `scan` is lazy, and an iterator's combinators one way

Status: Accepted. Extends [0128](0128-iterator-sources.md) and
[0139](0139-lazy-chains.md).

Case: C ([0262](0262-when-rust-and-js-disagree.md)): which closures run
would change, so Rust's is kept.

## Context

`scan` of a JS iterator listed it first, `$scan(it.toArray(), init, f)`:
a `map` before it ran its closure for every item, where Rust's lazy
`Scan` stops stepping at its first `None`. `v.iter().map(log).scan(..)`
logged items Rust never reached. Of items with a destructor, `scan` was
refused.

And `iter_comb`, which lowers `zip`, `chain`, `take_while`, `skip_while`,
`flatten`, `scan` and the rest, took whether its items were an array or a
JS iterator, `lazy`, and had a form for each, `$takeWhile` beside
`$lazyTakeWhile`, `concat` beside `$lazyChain`; but its one caller, MIR's,
always gave a JS iterator. The eager forms were reached by nothing: two
ways to do each job.

## Decision

**`scan` is `$lazyScan(it, init, f)`**, a generator stepping `it` only as
far as it gives, its `Some`s taken out of their boxes (ADR 0515), holding
its source closed to drop what it holds, as `take_while` does.

**`iter_comb` lowers a JS iterator's combinators, one way**: its `lazy`
flag and each eager form are gone, with the runtime's `$takeWhile`,
`$skipWhile` and eager `$scan`, and the mutations that switched between
them.

## Why

- **It's the same program**: `scan_lazy_drops` scans a logging map, items
  with a destructor and `Option`s of `Option`s, against native Rust.
- **One way**: what nothing reached is gone, not kept beside what is.

## Consequences

- A scan of an array's items is `$lazyScan($iter(v), ..).toArray()`,
  where it was `$scan(v.slice(), ..)`: `sales_report` and the `versions`
  snapshot.
- Mutations list the chain first, go on past `None`, keep the boxes, and
  refuse a scan of items with a destructor.

## Amendment: an object state is given as itself

`$lazyScan` gave its closure a box of its state, `{ value: init }`,
whatever it is, but a closure taking `&mut` of a struct, a tuple or a
`Vec` reads it as the object, `acc.sum` of the box (#134). The state's
`&mut` is as `handles_mut` says one is: a box of a number or text, the
object itself of the rest, which `$lazyScan` is told, `false` after the
closure. Held by `test/corpus/scan_object_state.rs` and the mutations
`scan-object-state-boxed` and `scan-handle-always`.
