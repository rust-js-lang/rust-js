# 0465. Dropping a std adapter is closing it, and what it holds is closed by its own drop

Status: Accepted. Extends [0422](0422-an-iterator-holding-a-value-is-closed-to-drop-it.md)
from `take` and `skip` to std's lazy adapters, and
[0421](0421-std-associated-type-drops-are-given.md); amends
[0190](0190-generic-no-destructor.md). Amended by
[0570](0570-a-closing-source-is-dropped-when-done.md): a source is dropped
when it's done, not only when it's closed.

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
