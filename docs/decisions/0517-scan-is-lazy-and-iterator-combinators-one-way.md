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
array's, which has nothing to close, steps on. (No longer: a `None`
closes nothing, and the source is closed as the adapter is, ADR 0465's
amendment "a zip holds its sides".) A std adapter over a
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

## Amendment: an inclusive range's, a reversed one's, and none of a `char`'s

`range_own_step` took any range's `nth` and `last`, a range of `char`s
too, whose `$rangeNth` added a number to a string: `('a'..'z').nth(2)` was
`"a2"`, and `last()` threw. A range of numbers alone is its bounds'; a
`char`'s steps by code point. `a..=b`'s `nth`, `last` and `count` are its
bounds' as `a..b`'s are, `$rangeInclusiveNth`, `$rangeInclusiveLast`,
`$rangeInclusiveLen`, where they built every number into an array. A
reversed one's `nth` is its `nth_back`, `$rangeNthBack` of its
`$iterator`'s bounds, `.of`, which tidy reads from where it's made; its
`last()` its start, `$rangeFirst`; and its `skip(n)`, where it's made with
a number end, that end `n` before; its `step_by`, its numbers from its
end, `$lazyRangeBack(a, b, n)`, of BigInts too (#163). Held by
`test/corpus/range_own_steps.rs` and `test/corpus/range_steps_known.rs`.

## Amendment: every adapter steps its source as std's does

An adapter over a source that gives `None` and then more is std's now,
not the difference this amendment left: every iterator the runtime makes
of Rust's is a `$Steps`, whose `map`, `filter`, `take` and `drop` pass a
`None` on ([ADR 0669](0669-an-adapter-steps-its-source-as-stds-does.md)).

## Amendment: a range's `nth`, wherever it comes from

A range's `step_by` and `skip` were its bounds' arithmetic only where
tidy's `range steps` pass saw the range made at the call; held in a
variable, given to generic code, of a start tidy couldn't read, reversed
or skipped first, it was stepped one number at a time, and `(0..u32::MAX)
.step_by(1 << 30)` never finished (#231). The rule is now where std's is:

- a range's JS iterator, `$iterator(r, $rangeNext)` or its inclusive or
  back form, has std's `Range::nth`, of its bounds (`$rangeNthOf`, a
  `char`'s by code point), and so does `$lazyRange`;
- a `step_by` of a range's numbers, a range's, a `rev()`'s or a
  `skip(n)`'s of one (`steps_a_range`), is `$stepBy`, which steps its
  source by its `nth`, and a `skip` is `$Steps`' `drop`, which does too;
- tidy's fold of `$stepBy($lazyRange(a, b), n)` into `$lazyRange(a, b, n)`,
  and of a reversed one into `$lazyRangeBack`, is a readability rewrite
  only: each of tidy's stages' JS finishes, as the nightly's tidy stages
  job asks. `as_double_ended` makes `$stepBy(range, n)` knowing its back,
  `$stepByEnds`, as it does the filter it was.

Tidy's folds of the `filter` of every `n`th a range's `step_by` was written
as, from its start and from its end, are gone: nothing writes it now. The
folds of `skip` stay, for how the JS reads; their mutations check
snapshots. `test/corpus/range_steps_anywhere.rs` holds it, with eight mutations. The
readability baseline's `Iterator.from` share grows by 0.1 in 10,000 lines,
the case's generic function's.

## Amendment: a `Scan` is closed whole, once (#741, #746, #748)

The build of std's iterator structs dropped the crate's iterator a
`scan()` held twice where something consumed the scan by value, `count`,
`sum`, `fold`, `last` or `for_each` (#741, #748): std's `Scan` has only
`try_fold`, so its `fold` is std's default, which takes the `Scan` and
drops it, and the compiled struct's fold did, but the caller dropped it
again, field by field, since `closed_on_drop`, the one list of what's
closed to drop (ADR 0465), left `Scan` out. Of a panic in its closure, a
collect dropped what it had collected before what was left of its source
(#746), as a struct dropped by its fields isn't closed as a collect
unwinds. The two are the one cause.

`closed_on_drop` now has `Scan`: its source's, then its function and its
state, as `Scan { iter, f, state }` drops them, of each that drops
something. A compiled `Scan` is closed whole, by the drop of each field
its `close` writes; one its fold took is closed already, so a consumer's
drop after drops nothing again (ADR 0912); and a collect closes it as it
unwinds, after the item the closure held.

In the build that rewrites adapters, `$lazyScan` drops its source as it's
closed, but never its state. A `scan` of a state with a destructor was
compiled there, its state never dropped, though Rust drops it with the
`Scan`; it's now refused, "`scan` of a state with a destructor", as one of
a function with a destructor already was, and the build of std's structs
drops it (results change: ADR 0262's third case). Giving `$lazyScan` the
state's drop is left to do.

A `scan` of the crate's iterator with a destructor, or of a `Vec`'s items
dropped unstepped, is no longer refused in that build: closed to drop, its
source is dropped as it's closed, as a `map`'s is. The refusal's tests,
`lowering.test.ts`'s and `diagnostics.test.ts`'s, are gone. `scan` was
the last adapter that reached three answers, whose mutations are taken
out with their tests: `drops/types.rs`'s refusal of a std adapter holding
what has a destructor that isn't closed to drop it
(`adapter-drops-as-struct`) and its drop of std's struct compiled for
this build by its fields (`compiled-struct-drops-refused`), and
`mir/iter.rs`'s drop of the crate's iterator an adapter holds as it's made
(`adapter-iterator-dropped`), beside its refusal of an adapter of one.
Whether anything still reaches them is asked apart, and what nothing
reaches is removed.
A state that rust-js drops nothing of, a `Vec<u32>`, isn't refused.

`test/corpus/scan_dropped_once.rs` checks each consumer by value, a held
scan, a `Vec`'s dropped unstepped and after a step, a panic in a count and two in a collect, from the front and
reversed, in both builds against native Rust; `iterator-forms.test.ts`
checks a state and a function with a destructor in the build of std's
structs. The mutations are `scan-not-closed-on-drop`,
`scan-state-not-closed` and `scan-dropping-state-unrefused`.

## Amendment: `$lazyScan` drops its function and its state (#760)

The build that rewrites adapters refused a `scan` of a state with a
destructor, which `$lazyScan` never dropped (#741's amendment), and one of
a function with a destructor, "kept by what it's given to". Both are
compiled now. `$lazyScan(items, init, f, handle, drop)` is given what
drops its function, then its state, given the state: run once, as it's
closed, after what it steps, as `Scan { iter, f, state }` drops its
fields. That is where Rust drops a `Scan`: at a consumer's end, after the
consumer's own closure, `ends` (ADR 0470), as a collect closes it, as a
panic unwinds and where it's held and dropped; and a state the closure
replaces is dropped by the write, the one left at the end by `drop`. A
scan's function isn't refused as kept by what it's given to:
`closed_on_drop` says the scan drops it as it's closed (`given_unused`).
The results changed where it was compiled, and it's refused no more: ADR
0262's third case.

`test/corpus/scan_drops_its_function_and_state.rs` checks each against
native Rust in both builds. Mutations: `lazy-scan-state-undropped`,
`lazy-scan-function-undropped`, `scan-kept-function-refused`,
`lazy-scan-drops-before-source` and `lazy-scan-drops-twice`. The
diagnostics test's refusal of a function a lazy scan holds is now one of
`from_fn`'s, which nothing closes to drop it; the guide's limitations
page lists the state's refusal no more.

## Amendment: a `step_by` of what no array's items bound is `$stepBy` (2026-11-09, #1046)

Since #1046 a `step_by(n)` of what no array's items bound
(`bounded_by_items`) is `$stepBy`, as its hint must be std's `StepBy`'s.
That one rule covers each it was before for its own reason: a source that
may give more after a `None` (#161), one whose `nth` reaches the crate's
own (#214), a range's numbers (`steps_a_range`, gone, #231), a zip's
(#654) and a flatten's (#668), none of which an array's items bound; their
five conditions, which no case reached apart from it, are gone, and so are
their mutations, one, `step-by-of-unbounded-as-filter`, holding the cases
each held. A filter of every `n`th stays for an array's items.
