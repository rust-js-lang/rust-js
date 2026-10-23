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

## Amendment: `scan` and `map_while` aren't fused

`$lazyScan` was a generator, which a `None` ended for good, and
`map_while` a map then a `take_while`, which stays ended; Rust's `Scan`
and `MapWhile` aren't fused, so stepping on after a `None`, `by_ref()`
then `collect()` again, steps the source on (#161). Each is an iterator
whose `next` steps its source each time, `$lazyScan` and
`$lazyMapWhile`; a `None` closes the source, as the generator's end did,
dropping what's left of one with a destructor (`$closing`), and an
array's, which has nothing to close, steps on. A std adapter over a
source that isn't fused, the crate's iterator that gives `None` then
`Some`, is fused, a JS iterator helper's: the guide lists it. Held by
`test/corpus/scan_map_while_unfused.rs`.

## Amendment: a range's steps are its bounds' arithmetic

A range made where it's stepped, `$lazyRange(a, b)`, stepped every
number for `step_by` and `skip`, and a range's `nth`, `last` and `count`
did too, where std's range knows them by arithmetic:
`(0..u32::MAX).step_by(1 << 30)` and `(0..u64::MAX).nth(1 << 31)` never
finished (#163). Tidy folds a lazy range's `step_by(n)` into
`$lazyRange(a, b, n)` and its `skip(n)` into a start `n` steps on; a
range's own `nth`, `last` and `count`, of `u64`s too, are its bounds'
(`range_own_step`): `$rangeNth`, which moves its start past what it
gives, `$rangeLast` and `$rangeLen`. A stepped range's `count()` steps
`n / step` numbers, as std's `StepBy` does; a reversed range's `nth`,
its own `nth_back`, still steps each. Held by
`test/corpus/range_steps_known.rs` and `test/corpus/range_own_steps.rs`.
