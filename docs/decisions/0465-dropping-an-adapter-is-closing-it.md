# 0465. Dropping a std adapter is closing it, and what it holds is closed by its own drop

Status: Accepted. Extends [0422](0422-an-iterator-holding-a-value-is-closed-to-drop-it.md)
from `take` and `skip` to std's lazy adapters, and
[0421](0421-std-associated-type-drops-are-given.md); amends
[0190](0190-generic-no-destructor.md). Amended by
[0570](0570-a-closing-source-is-dropped-when-done.md): a source is dropped
when it's done, not only when it's closed.
Amended by [0992](0992-eq-of-trusted-len-iterators-asks-their-lengths.md):
`eq()` of two `TrustedLen` iterators of unequal lengths compares none.

Case: C ([0262](0262-when-rust-and-js-disagree.md)) for what's dropped and
how many times; B for when, where an adapter ends before what it steps,
listed below.

## Context

itertools adapts what it's given, in generic code, of items its caller
may give a destructor: `i.into_iter().zip(j)`, `chain`, `take`, and
`a.into_iter().eq(b)`. A `map`, a `filter`, a `zip` of a `Vec`'s items with
a destructor, dropped before they're all stepped, drops what they haven't
given, and each item it discards, `take_while`'s and `step_by`'s, as it
discards it. JS's helpers and rust-js's generators hold what they step
where nothing can reach it, so each was refused, but `take` and `skip` of
an owning source (ADR 0422).

## Decision

**A std lazy adapter, `map`, `filter`, `enumerate`, `take`, `skip`,
`skip_while`, `take_while`, `step_by`, `inspect`, `fuse`, `zip` and
`chain`, is closed to drop: its drop is `it.return()`, which closes what it
steps. What it holds is closable: an iterator that owns its items, a
generic one, a crate's own, each `$closing(it, drop)`, closed by its own
drop, once; or one that holds nothing, a slice's or a range's.**

```js
const it = $closing($iter(v), $dropEach(nDrop_drop)).take(2);
return $lazyZip($closing(tmp, dropIIntoIter), $closing(Iterator.from(j), dropJ), dropIItem);
```

- **Which are closed to drop is `recognition::closed_on_drop`'s one
  list**, of what each source holds: `closing_source`. An adapter that holds
  its sources is `IterMethod::holds_its_source`'s and
  `IterComb::holds_its_sources`'s; each wraps them with `closing`, whose
  drop is `closing_drop`'s: an owning one's, `$dropEach(drop)`, each item it
  hasn't given; a generic one's, its caller's, `dropIIntoIter` (ADR 0421);
  the crate's, its own, of what `$iterator` steps.
- **Each discards as Rust does**: `filter`'s and `skip_while`'s, and
  `take_while`'s item it stops at, dropped as they're tested; `step_by`'s
  skipped; `skip`'s, `$skipDropping`; and `zip`'s item it stepped where the
  other's run out. `eq` drops each pair it compares, the other's first, and
  closes both as it ends.
- **rust-js's own adapters close what they hold**: `$lazyZip` its other
  side, `$lazyChain` both, as JS's helpers close what they step.
- **What drops each item an iterator hasn't given is `$dropEach(drop)`**,
  as callers give generic code it, where it was an arrow of a loop.
- **A chain of `map`s and `filter`s run over every item never closes** what
  it steps: an array's, as before, `v.filter(f)` (ADR 0055).

## Why

- **It's std's**: `adapter_drops` steps each adapter of a `Vec`'s items with
  a destructor once and drops it, in concrete code and in generic code given
  them, a crate's iterator with a destructor of its own too, with `eq`, and
  prints each drop where native Rust does.
- **It's JS's own disposal**, as ADR 0422's: `return()`, which a `for` loop
  and JS's helpers call when they end early.

## Consequences

- **An adapter that ends before what it steps closes it as it ends**, as
  ADR 0422's `take` does: `take_while` that stops, `zip` whose side runs
  out, drop what's left then, not when they're dropped (case B).
- **`eq` steps both**: std compares the lengths of two `TrustedLen`
  iterators first, two `Vec`'s, and steps neither where they differ; rust-js
  steps both till the shorter ends, the same answer, a `PartialEq` that
  prints, or the order a destructor's run in, telling them apart (case B).
- **ADR 0190 shrinks again**: generic code's `map`, `zip` and the rest of
  what its caller gives with a destructor compile; a `peekable`, `rev`,
  `flat_map` or `scan` of one, which hold an item, or step from the back,
  are refused still.

## Amendment: `successors` of a value with a destructor (2026-10-22)

`iter::successors(first, f)` of a value with a destructor was refused. It
holds the next item it gives, as Rust's `Successors` does: `$successors`
is an iterator, not a generator, whose `return()` drops what it holds,
its item's drop given where it's made, so one closed before its first
step drops its first item too, which a generator's `finally` wouldn't.
Each item it gives is the consumer's. `closed_on_drop`, the one list of
what's closed to drop, names it beside `repeat`. `successors_drops`
holds `count()`, `find()` and one never stepped against native Rust.

## Amendment: the crate's iterator with a destructor, stepped from its back

An iterator of the crate's with a destructor, mapped and then reversed,
`count().map(f).rev().sum()`, gave 0 where Rust gives 90 (#223): the map,
made first as `$closing($iterator(count(), next), drop)`, was remade
knowing its back by `rev()`, and `user_iterator_knowing` wrapped that
JS iterator as if it were the crate's value, whose methods were then
called with the `$closing` as `self`. `closed_parts`, beside
`stepped_value` the one reader of `$closing(it, drop)`, lets
`user_iterator_knowing` make the iterator inside knowing its back, still
closed to drop it. And a `rev()` of such an iterator, and an
`enumerate()` of one stepped from its back, are closed to drop it, by
the type `rev()` is given, so a `&mut` of it isn't: their temporaries
were never dropped. ADR 0262's case: C, a wrong value, fixed.
`test/corpus/own_iterator_dropped_from_back.rs` holds them, with four
mutations; what else of such an iterator isn't dropped, through a
`step_by`, a `filter` or a `skip` stepped from its back, is #225. The
readability baseline's `while (true)` share grows by 0.1 in 10,000
lines: the case's `rfold`, a `while let`.

## Amendment: a `Vec`'s `IntoIter` drops what it passes from its back as one slice (#234)

std's `vec::IntoIter::advance_back_by(n)` drops the `n` items it passes
as one slice, in the order they're in the `Vec`. `array::IntoIter`'s,
`VecDeque`'s and `BinaryHeap`'s do the same. A `Drain`'s and a B-tree's
`IntoIter` use the default, which drops each item from the back as it
passes. So `vec![D(1), D(2), D(3), D(4)].into_iter().rev().step_by(3)`
drops 2 then 3, where the JS dropped 3 then 2. Six of seven fuzz failures
in seeds 44000–44499 were this.

- **The type says it** (`drops_passed_as_slice`, recognition). Its
  `$closing` is `$closing(it, drop)`, whose `nthBack` drops a slice; one
  that owns an array's items and drops each as passed is `$closing(it,
  drop, false)` (`closed_by`, the one builder, and `passes_each_owned`).
  `$nthBack` of a plain `$iter` takes the same `false`. A `Vec`'s, the
  common case, reads as before.
- **What asks its `nth_back` reaches it**, as std's adapters ask: `$rev`'s
  `nth`, which says it drops what it passes (`ownNth`); `$skipping`'s
  first step and its `nth`, std's `Skip::next` and `Skip::nth`, of its
  source's `nth`; and `$stepBy`, which a `step_by` of a reversed `Vec`'s,
  or of a `step_by` or `skip` of one (`nths_a_slice`), is lowered to in
  place of the filter, which drops each as it passes.

`$stepByEnds` gains std's `StepBy::nth_back`: one `nth_back(n * step +
next_back_index)` of its source, which `rev().nth(k)` asks, where it
stepped its back `k + 1` times (#245).

A `chain` of a reversed `Vec`'s skipped, `rev().chain(..).skip(2)`,
still drops each as it passes, since `$lazyChain` has no `advance_by`
that passes a slice on (#250). Held by
`test/corpus/vec_skipped_from_back_drops.rs`, whose `Drain` drops each
from its back, `test/corpus/step_by_nth_back.rs`, and eleven mutations.
The droppable sweep's 2,100 programs agree; 80 that were refused now
compile, where a reversed `Vec`'s `step_by` is `$stepBy`. The readability
baseline's `while (true)` share grows by 0.2 in 10,000 lines and its
`$iter` share by 0.5. Both come from the new case, whose three `for`
loops each drop what they're given, over its eight `Vec`s.

## Amendment: a `chain` asks each side as std's `Chain` does (#232, #250)

- **`last`** (`$lastDropping`) of a `chain` is each side's own `last`,
  `a`'s then `b`'s, each dropping what it passes. Where `b` has a last,
  `a`'s is dropped then, as `b_last.or(a_last)` drops it. A `skip`'s is
  its source's after what it skips (`skipped`), as std's `Skip::last` is
  `iter.nth(n - 1)?` then `iter.last()`. `vec![D(16), D(17)].chain(vec![
  D(18), D(19)]).last()` drops 16, 18, then 17.
- **`nth`** is std's `Chain::nth`, `a.advance_by(n)` then `b`'s, and
  `$lazyChain` gains std's `advance_by`. A `Vec`'s `IntoIter` drops what
  it passes as one slice from either end, so its `$closing` gains
  `advance_by` and `nth` from its front (`ownNth`), and `$rev` maps
  them across. A chain whose two sides drop what they pass is `ownNth`
  itself, so `$skipping` and `$nthDropping` ask it.
- An array given to `chain` or `zip` is a `$iter` of it, as std's
  `IntoIter` steps it, where it was `Iterator.from(xs)`.
  `adapter_drops.js` and `last_option_drops.js` read so now.

Held by `test/corpus/chain_sides_own_steps.rs` and eight mutations.
`$rev`'s `source`, which nothing read since a held `Rev` became std's
compiled one, is gone, along with its mutation. The readability
baseline's `$iter` share grows by 1.7 in 10,000 lines: 4 are the chain
arguments, whose `Iterator.from` share falls by 0.4, and the rest are
the new case's sixteen `Vec`s. Its renamed `x$1` share grows by 2.8,
the new case's temporaries, each a `Vec` made in a `try` that drops
what came before.

## Amendment: a `skip`'s fold and `try_fold` are std's (#250)

std's `Skip::next` asks its source's `nth(n)`, while its `fold` and
`try_fold` ask `nth(n - 1)` and then go on with their source's own,
dropping the item that `nth` gives. For a reversed `Vec`, whose `nth`
drops a slice, the order differs. `rev().skip(2).any(..)` of `[22, 23,
24, 25]` drops 25, then 24, not 24 and 25 together (fuzz seeds 45192
and 45321). `$skipping` gains both, where its source drops what it
passes itself. Held by `vec_skipped_from_back_drops.rs`'s fold and
`any`, and the mutations `skipping-fold-by-next` and
`skipping-try-fold-by-next`. The readability baseline's `$iter` and
renamed `x$1` shares each grow by 0.2 in 10,000 lines, from the two new
lines' `Vec`s, each made in a `try` that drops what came before.

## Amendment: a `take` and a `skip` pass a reversed `Vec`'s slice on (#261)

std's `Take::count` is its source's `advance_by(n)`, and `Skip::count`
is `nth(n - 1)` and then its source's `count`. Of a reversed `Vec`, each
drops what it passes as one slice, so `rev().take(2).count()` of `[1, 2,
3]` drops 2, 3, then 1. Such a `count` is `$count`, which asks each
adapter's own, where the iterator is a `Take` or a `Skip` whose `nth`
drops a slice (`nths_a_slice`, which now also passes through `Take`).
`$skipping` gains std's `count`. A source that drops what it passes
says each item's drop too (`dropItem`), so a `take` of it is `ownNth`.
Past its end, a `take`'s `nth` drops the item its source's `nth(n - 1)`
gives, as std's `Take::nth` does. Fuzz seeds 45520 and 46382 agree.
`Fuse` and `Peekable`, compiled from std, don't pass `nth` on yet
(46168, 46397, 45792; #261 stays open). Held by
`vec_skipped_from_back_drops.rs` and four mutations. That case's
`filter(..).skip(2)` now holds `skip-dropping-keeps-skipped`, whose old
case no longer steps by `next`. The readability baseline's `$iter` and
renamed `x$1` shares each grow by 0.5 in 10,000 lines, both from that
case's five new lines: their `Vec`s, and the temporaries made in each
`try`.

A `Chain` with such a side is `nths_a_slice` too, so its `step_by` is
`$stepBy`, which asks the chain's `nth`, std's `advance_by` of each side
(fuzz seed 46975). Held by `chain_sides_own_steps.rs`'s `step_by(3)` and
the mutation `chain-nths-no-slice`. That line's two `Vec`s grow the
readability baseline's `$iter` share by 0.2 in 10,000 lines and its
renamed `x$1` share by 0.5, each made in a `try` that drops the other.

A chain's `nth` that advances a side with no `advance_by` of its own, a
`skip`'s, drops what it passes by that side's `dropItem`. Before this,
`skip(0).chain(..).nth(2)` dropped none of the two it passed (#269,
fuzz seed 49156). A chain that's `ownNth` says its own `dropItem` too.
Held by `chain_sides_own_steps.rs` and the mutation
`chain-side-advanced-undropped`. That line's two `Vec`s grow the
readability baseline's `$iter` share by 0.2 in 10,000 lines and its
renamed `x$1` share by 0.6.

## Amendment: a `step_by` over a `Peekable` or a `Fuse` asks its `nth` (#261)

`nths_a_slice` now also passes through `Fuse` and `Peekable`, whose
`nth` std takes from their source. A `step_by` over one of a reversed
`Vec` is now `$stepBy`, which asks its `nth`, not a `filter` that drops
each item it passes back to front. `$closing` now passes on its
iterator's `ownNth`, a compiled std struct's, so `$stepBy` asks it.
Together with #276, the `nth` reaches the `Vec`'s `nth_back`, which
drops what it passes as one slice, front to back (fuzz seeds 48170,
46397, 45792, 47981 and 48433). This follows ADR 0262's "results change"
case. `Enumerate` isn't on the list, because the JS `enumerate` is a
`map` with no `nth` of its own (#277). The `step_by_compiled_source_nth`
corpus case checks against native Rust. The mutations
`nths-a-slice-not-through-peekable`, `nths-a-slice-not-through-fuse` and
`closing-own-nth-unsaid` are caught.

Readability: the case's `Vec`s of values with a destructor add `$iter`
(77.8 to 77.9 in 10,000 lines) and renamed temporaries (716.7 to 718.3),
as each such case does. The output reads no worse.

## Note: a zip's sides, stepped after its first end (#279)

`restarting_iterators` now zips its flaky iterator with a `Vec` of values
with a destructor, collected twice, which holds `zip-closed-at-end`: a
zip closes its sides only as it's closed. Readability: the `Vec`'s
`$iter` adds to that share (77.5 to 77.6 in 10,000 lines), as each such
case does.

## Amendment: a chain's `count` is each side's own (#283)

`$lazyChain` had no `count`, so `$count` stepped each item. std's
`Chain::count` is `a.count() + b.count()`, and `Take::count` advances its
source: a reversed `Vec`'s `advance_back_by`, which drops what it passes
as one slice, front to back. Following ADR 0262's "results change" case,
Rust's order is kept: the chain's `count` is each side's `$count`, `a`'s
then `b`'s (fuzz seed 53335). `chain_sides_own_steps` counts a reversed
`Vec`'s `take(2)` chained to another, against native Rust. The mutation
`chain-count-stepped` is caught.
Readability: its `Vec`s of values with a destructor add `$iter` (77.6 to
77.8 in 10,000 lines) and renamed temporaries (714.4 to 714.6), as each
such case does.

## Amendment: an `enumerate()`'s `nth` is its source's too (#277)

`nths_a_slice` now passes through `Enumerate` as well. An `enumerate()`
whose output's `nth` drops a slice is now `$enumerate`, whose `nth` asks
its source's, not JS's `map`. It passes on its source's `ownNth`, so a
`step_by` over it asks it. `step_by_compiled_source_nth` adds
`rev().enumerate().step_by(3).nth(1)`. The mutations
`nths-a-slice-not-through-enumerate`, `enumerate-own-nth-unsaid` and
`enumerate-of-a-slice-nth-mapped` are caught.
Readability: the line's renamed temporaries add to that share (714.6 to
715.5 in 10,000 lines), as each such case does.

## Amendment: an `nth` is given the items' drop, which a chain's side may lack (#285)

Since `$closing` passes on a compiled struct's `ownNth` (#261), a chain
of a `fuse()` and a `Vec` took its own `nth`. That `nth` advances the
first side by that side's `dropItem`, which a closed compiled struct
doesn't have. So `fuse().chain(..).nth(2)` never dropped the two items
it passed, a value whose destructor never ran. `$nthDropping` now gives
an `ownNth` iterator the items' drop, `$lent` passes it on, and a chain
advances a side with no item drop of its own by it. Stepping from the
front drops what it passes front to back, as std's `advance_by` does.
`chain_sides_own_steps` adds the case, against native Rust. The mutations
`nth-dropping-drop-ungiven`, `lent-nth-drop-unpassed` and
`chain-nth-given-drop-unused` are caught.
Readability: the line's `Vec`s add `$iter` (81.9 to 82.1 in 10,000
lines) and renamed temporaries (713.5 to 714.1), as each such case does.

## Amendment: a compiled `Fuse`'s and `Rev`'s `advance_by` is std's (#286)

std's `Chain::nth` asks its first side's `advance_by`. Through
`take(2)`, a held `fuse()` and a `rev()`, that reaches a `Vec`'s
`advance_back_by`, which drops a slice front to back. The compiled
structs had no `advanceBy`, so the JS stepped each item from the back.
Now `std_iterator_steps` gives a `Fuse` `$fuseAdvanceBy` (its source's,
let go on a shortfall unless it's a `FusedIterator`) and a `Rev`
`$revAdvanceBy` (its source's `advance_back_by`). `$stdIterator` exposes
it, and `$closing` forwards a struct's own. The one-slice drop is
`$advanceSlice`, shared with `$closing`'s `Vec` steps. And a `take` is
its own `nth` wherever its source is, using the item drop it's given
when its source has none (#285). Fuzz seeds 55134 and 55183.
`chain_sides_own_steps` adds both shapes, against native Rust. The
mutations `rev-advance-by-stepped`, `closing-advance-unforwarded`,
`take-own-nth-needs-item-drop` and `fuse-advance-by-unlisted` are caught.
Readability: the lines' `Vec`s add `$iter` (82.1 to 82.4 in 10,000
lines) and renamed temporaries (714.1 to 715.5), as each such case does.

## Amendment: an explicit `skip`'s `advance_by` is std's (#287)

std's `Skip::advance_by(n)` is its source's `advance_by(skip + n)`, asked
once. For a reversed `Vec`, that's one slice, front to back. `$skipping`
had no `advanceBy` of its own, so a chain's `nth` past
`rev().skip(2)` stepped each item from the back (fuzz seed 56298). It
now asks its source through `$advanceByOf`. `chain_sides_own_steps` adds
the case, and `skipping-advance-by-stepped` is caught.
Readability: the line's `Vec`s add `$iter` (85.1 to 85.3 in 10,000 lines)
and renamed temporaries (729.4 to 730.5), as each such case does.

Note (#285): since a side's item drop falls back to the other side's
(#286), the drop an `nth` is given matters only where neither side has
one, a chain of two `fuse()`s. `chain_sides_own_steps` adds that case,
which holds `nth-dropping-drop-ungiven`, `lent-nth-drop-unpassed` and
`chain-nth-given-drop-unused` again. They had survived main's check of
6d8613336. Readability: `$iter` 85.3 to 85.4 and renamed temporaries
730.5 to 731.8 in 10,000 lines, as each such case does.

## Amendment: a `step_by` gives the `nth` it asks the items' drop (#289)

`$stepBy` asks an `ownNth` source's `nth`. A `take` over a `step_by` is
one, and past its end it drops the item its source's `nth(n - 1)` gives,
using the drop it's given, since it has none of its own (#285). `$stepBy`
gave none, so `rev().step_by(2).take(2).step_by(3)` never dropped that
item (fuzz seed 60523). It now passes its drop. Other callers that ask
an `nth` and have a drop, `$skipping`, `$closing`, a chain's second side
and `$rev`'s back, would pass it the same way. No case reaches them yet,
so they get it with the case that does. `step_by_compiled_source_nth`
adds the seed's shape, and `step-by-nth-drop-ungiven` is caught.
Readability: the line's renamed temporaries add to that share (735 to
736.1 in 10,000 lines), as each such case does.

## Amendment: a compiled `Peekable`'s and `Fuse`'s `count` is std's (#293)

std's `Peekable::count` is what it peeked at, then its source's
`count`. `Fuse::count` is its source's. Through `take(2)` of a reversed
`Vec`, that's `Take::count`, which advances the source and drops one
slice front to back. The compiled structs had no `count`, so `$count`
stepped them back to front (fuzz seed 62344). `std_iterator_steps` now
gives them `$peekableCount` and `$fuseCount`, and `$stdIterator` exposes
them. `chain_sides_own_steps` counts both, and `peekable-count-unlisted`
and `fuse-count-unlisted` are caught.

## Amendment: a `skip`'s first step asks its source's `nth`, std's default too (#294)

std's `Skip::next` asks its source's `nth(n)`. Where that source has no
`nth` of its own, std's default advances by `try_fold`. Two shapes
dropped a reversed `Vec`'s items back to front (fuzz seeds 63126 and
63028):
- `rev().skip(2).inspect(..).step_by(2).count()`: the JS `step_by` was a
  `filter`, whose consumer folded it into `Skip::fold`. Its source's
  first step is a `Skip`'s `nth` past a slice, `steps_past_a_slice`, so
  it's now `$stepBy`, stepped by `next` as std's `StepBy` steps it.
- `rev().skip(2).take_while(..).skip(2)`: `$skipping` stepped a
  `take_while` by `next`. Now it asks the `nth` of a source with its own
  `tryFold`, whose default, `advanceBy` by `tryFold`, reaches the inner
  `skip`'s `nth(n - 1)`. `$Steps`' `nth` is given the items' drop for
  what it passes.

`step_by_compiled_source_nth` adds all three lines, against native Rust.
The mutations `step-by-past-a-skip-filtered`, `skipping-nth-only-own` and
`steps-nth-drop-ungiven` are caught.
Readability: the lines add `$iter` (87.6 to 87.7 in 10,000 lines),
`Iterator.from` (15.3 to 15.4) and renamed temporaries (732.9 to
735.1), as each such case does.

## Amendment: `last` folds an adapter with its own `fold` (#299)

std's default `last` is a `fold`. For `rev().skip(2).take_while(..)`,
that reaches `Skip::try_fold`: `nth(1)` (one slice) and then the next.
`$lastDropping` stepped `next` instead, `Skip::next`'s `nth(2)`, a slice
of two, which drops in another order (fuzz seed 68239). It now folds an
iterator with a `fold` of its own, keeping the last item and dropping
each before it. `step_by_compiled_source_nth` adds the shape, and
`last-steps-not-folds` is caught.
Readability: the line adds `$iter` (87.7 to 87.8 in 10,000 lines) and
renamed temporaries (734.3 to 735), as each such case does.

## Amendment: an `nth` of an adapter with its own `try_fold` is std's default (#302)

`$nthDropping` stepped `next` for an iterator without its own `nth`.
std's default `nth` is `advance_by`, which a sized iterator runs by
`try_fold`. For `rev().skip(2).inspect(..).nth(3)`, that reaches the
`skip`'s `nth(1)` and then the next, not `Skip::next`'s `nth(2)` (fuzz
seed 73125). `$nthDropping` now asks any iterator with an `nth`. A
`$Steps`' default `nth` is `advanceBy` by its `tryFold`, and since #294
it's given the drop, so it drops what it passes. The old mutation
`nth-dropping-default-nth`, whose premise was that it didn't, is gone. `step_by_compiled_source_nth` adds the shape, and
`nth-dropping-default-stepped` is caught.
Readability: the line adds `Iterator.from` (15.2 to 15.3 in 10,000 lines)
and renamed temporaries (729.4 to 730.4), as each such case does.

## Amendment: a zip holds its sides, and no adapter closes its source as it ends

A `zip` whose other side ended at a `None` of its own dropped what that
side had left there, then the item it had stepped of `a`, then what `a`
had left: `a.into_iter().map(..).zip(b.into_iter().take_while(odd))`
printed `drop 2`, `drop 5`, `drop 12`, `drop 13` where native Rust prints
`drop 2`, `drop 12`, `drop 13`, `drop 5`. Results change (case C, not
this ADR's case B): std's `Zip` owns its sides, and `TakeWhile`, `Scan`,
`MapWhile` and `Take` drop nothing as they end; what they hold goes as
the zip is dropped, `a`'s then `b`'s. A `scan`'s, a `map_while`'s, a
`take`'s of a `filter` and a `skip` past them did the same.

- **`$lazyZip` holds both sides**, `hold()`, as a variable's zip already
  did: one that ends doesn't close itself there, and the zip's `close`
  closes `a` then `b`, as std drops them.
- **`$lazyScan`, `$lazyTakeWhile` and `$lazyMapWhile` no longer close
  their source at their own end**, by `next` or by `try_fold`. Their
  consumer closes them where Rust drops them, as JS's own stepping,
  `fold`, a `chain`'s first side and every adapter's `return()` do; the
  close at their end could only drop earlier than Rust. Of the corpus'
  21 cases of these helpers and a probe of each by-value consumer of a
  `scan` of values with a destructor (`max`, `min`, `last`, `count`,
  `sum` of `Option`s, `eq`, `cmp`, `partition`, `unzip`, `reduce`,
  `for_each`, `is_sorted`, a `Result`'s `collect`, `skip`, `filter`,
  `chain`, `step_by`, `enumerate`), every one prints what native Rust
  does without them. Their mutations go: `scan-past-none`, which main's
  check found surviving, and `take-while-held-closes` and
  `map-while-held-closes`, whose code is gone.
- `test/corpus/zip_sides_dropped_with_it.rs` holds it against native
  Rust, with the new mutation `zip-sides-unheld`, and catches
  `zip-other-unclosed` and `zip-closed-at-end`, which survived too: with
  its sides held, the zip's `close` is what drops `b`'s, and a zip a
  variable holds, stepped to its end, drops both where the variable is
  dropped.

A `take`'s close at its limit, `$Steps.take`, stays: a `zip`'s holds it,
and nothing else seen steps one past it before its consumer drops it.
Readability: the case's lines add `$iter` (83 to 83.7 in 10,000 lines),
renamed temporaries (677.8 to 681.4), each `let`'s drops of what a panic
would leave, and its `while let`'s `while (true)` (10.4 to 10.5), as each
such case does.

## Amendment: a panic stops a chain early (2026-10-30, #422)

A chain of `map`s and `filter`s over an owning source, or a consumer that
takes each item, `for_each`, `fold`, runs over every item, so its
`$closing` was dropped, or never made: unless a closure panics, which
stops it early. Rust then drops, as it unwinds, what the panicking closure
held, what a `collect` had collected, and what's left of the source; JS
dropped the first alone, `drop 3` where native drops 3, 1, 2, 4. Results
change, so Rust's behavior is kept (ADR 0262):
- **A consumer that runs code as it takes each item of an owning source
  is given it closed** (`dropping_source`), not a `collect()`, whose
  adapters close theirs already, nor a type parameter's, which MIR's own
  drop after the call drops, unwinding too.
- **`$Steps.fold`, and `$closing`'s `fold` and `rfold`, close it though
  their function throws**, as Rust's `fold` drops what it took.
- **A `collect` into a `Vec` a stage's panic may stop
  (`chain_may_throw`), from an owning source (`closes_a_source`) or of
  items with a destructor, is `$collectDropping(it, drop)`**: what it
  collected dropped, then its source closed; and closed at its end too, as
  `Array.from`'s iteration was.
- **`$extend` closes what it's given where stepping it throws.**
- **The eager forms keep `$closing` where a stage, or the function taking
  them, may throw** (`in_place::may_throw`).
14 corpus cases read so, 89 lines more, an owning source's `$closing`
where a closure runs, and drop bodies given as a function where one's
passed. Held by `panic_mid_iteration_drops.rs` against native Rust and the
mutations `closing-fold-unclosed-on-throw` (`runtime/closing.ts`),
`collect-dropping-keeps-collected` (`runtime/fold_of.ts`),
`extend-unclosed-on-throw` (`runtime/extend.ts`),
`consumer-source-unclosed` (`lower/mir/iter/closing.ts`),
`eager-taker-throw-unseen` (`lower/iterators/eager.ts`) and
`collect-unwinding-unseen` (`lower/mir/iter/collect.ts`). `docs/readability.txt`'s
`$iter` share went from 82.1 to 82.4 in 10,000 lines, blessed: each a
source that keeps its `$closing`, `$closing($iter(v), ..)`, where a closure
may panic, and the new case's own.

## Amendment: each frame a panic unwinds drops its own (2026-10-30, #423)

#422 left the shapes past a `Vec`'s collect, a `fold` and an `extend`.
Native, the panicking closure's own item comes first, as its frame
unwinds, a borrowing predicate's too. Of a `Vec` std collects in place,
what it collected comes next, then what's left; otherwise what's left
comes before what it collected, the fold's own frame inside the `Vec`'s.
Results change, so Rust's behavior is kept (ADR 0262):
- **A predicate that may throw drops the item it was asked of:**
  `filter`, `take_while` and `skip_while` (`dropping_discarded`'s
  `let kept = false; try { kept = keep(item); } finally { if (!kept)
  drop(item); }`), and `inspect`. Not one of the crate's functions that
  never unwinds, whose reference is `Known::NeverThrows` (ADR 0902).
- **A runtime consumer's callback that throws drops what it was given,**
  then its fold closes what's left: `$extremeBy`'s comparison drops the
  item, then the one kept; `$extremeByKey`'s key the item; `$partition`'s
  predicate the item, then `no`, then `yes`. Each takes a `Vec`'s items it's
  given closed. `$lastDropping` drops the one it kept.
- **A collect a panic stops drops what it collected and what's left in
  std's order:** `recognition::collects_in_place` is alloc's
  `in_place_collectible`, read by trait selection (`InPlaceIterable` and
  `SourceIter` of an `AsVecIntoIter`) and the items' layouts. Where it's
  false, of types rustc knows whole, `$collectDropping(it, drop, true)`
  drops what's left first (`$collectUnwound`). A hash map or set, or a
  `LinkedList`, extends one as each comes, so what's left comes first too.
  `$collectResults`, `$collectOptions` and `$foldToArray` (a compiled std
  struct's, a `zip`'s) do the same. A B-tree, a `BinaryHeap` and a
  `String` collect by `$collectDropping`. A new hash map or set is
  `$mapFrom`, whose map is dropped after what's left, and `$extendMap`
  closes what it's given where stepping it throws.

Still open in #423 are three things:
- a consumer's accumulator held inside an adapter's fold when a stage
  throws: a `fold`'s, a `last`'s, a `max_by_key`'s;
- the order JS gives those, what's left before the accumulator;
- a stage a variable holds, which `chain_may_throw` can't see.

7 corpus cases read so, 29 lines more: a `true` where what's left drops
first, a drop given to `$partition`, a `$mapFrom`, and a predicate that may
throw in its `try` form. Held by `panic_part_way_drops.rs` against native
Rust and the mutations `discarded-throw-keeps-item` (`lower/iterators/ends.ts`),
`inspect-throw-keeps-item` and `partition-given-no-drop`
(`lower/combinators/adapters.ts`), `extreme-compare-throw-keeps-both`,
`extreme-key-throw-keeps-item` and `extreme-array-rest-kept`
(`runtime/max_by.ts`), `partition-throw-keeps-sides`, `last-throw-keeps-last`,
`collect-rest-first-ignored` and `fold-to-array-unwinds-nothing`
(`runtime/fold_of.ts`), `map-from-keeps-entries` and
`extend-map-unclosed-on-throw` (`runtime/map_ops.ts`),
`collect-results-unwinds-nothing`, `collect-options-unwinds-nothing`,
`in-place-collect-any-size` (`lower/recognition/specialization.ts`),
`rest-first-never`, `map-collect-unwinding-unseen`,
`sorted-collect-unwinding-unseen`, `string-collect-unwinding-unseen`,
`heap-collect-unwinding-unseen` and `folded-collect-given-no-drop`
(`lower/mir/iter/collect.ts`), `fallible-rest-first-dropped`
(`lower/iterators/stepping.ts`) and `never-unwinding-predicate-may-throw`
(`lower/iterators/in_place.ts`).
