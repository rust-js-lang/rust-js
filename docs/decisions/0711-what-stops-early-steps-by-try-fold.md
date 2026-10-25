# 0711. What stops early steps by `try_fold`, and `nth` is passed on, as std's adapters route them

Status: Accepted. Amends [0687](0687-a-consumer-steps-its-iterator-as-std-does.md)
and [0690](0690-an-iterator-given-as-it-is-folds-by-its-own-fold.md), whose
consumers that fold were std's, but not those that stop early.

Case ([0262](0262-when-rust-and-js-disagree.md)): R: results change, so
Rust's, paid for in the runtime.

## Context

A generated program (seed 2718314) printed `nth count 3` where Rust
printed `nth count 2`:

```rust
Count { at: 0, end: 4 }.skip(3).filter(|_| true).any(|_| false)
```

`any` is std's `try_fold`, and std's adapters pass it on: `Filter`'s to its
source, and `Skip`'s is its source's `nth(n - 1)`, then its source's
`try_fold`. A `skip`'s `next` is its source's `nth(n)` instead. The
runtime's `$Steps` had std's `fold` and `nth`, but `some`, `every` and
`find` were JS's, stepping by `next`, so an iterator that writes its own
`nth` was asked the wrong `n`. The same was true of:

- `position`, `find_map`, `try_fold`, `try_for_each`, and `eq`, `cmp` and
  the rest, std's `iter_compare`, which steps its left side by
  `try_for_each`;
- `chain`, `take`, `take_while`, `map_while`, `scan`, `flat_map` and
  `skip_while`, whose `try_fold` is their source's;
- `filter`'s and `skip_while`'s `next`, which is their source's `find`;
- `skip`'s `nth(k)`, its source's `nth(n + k)`;
- `$closing`'s `nth`, which stepped where its iterator's own was there.

Tracing it also showed `iter.eq(3..21)` threw a `TypeError`: `eq`'s other
side was given as its value, a range's `{ start, end }`, not as an
iterator.

## Decision

**`$Steps` has std's `try_fold`, `tryFold(init, f, broke)`, where `broke`
is `Try::branch`: whether what `f` gave stops it. What stops early asks
it, and each adapter passes it on as std's does.**

- **Its default** steps `next`, as std's does. No crate's iterator writes
  its own: `Try` is unstable.
- **Asked by** `some`, `every`, `find`, `$position`, `$tryFoldSome`,
  `$tryFoldOk`, `$iterEq` and `$iterCmp`, through `$lent`, which gives its
  iterator's.
- **Passed on** by `map`, `take`, `flatMap` (the rest of the inner one it's
  in, then its source's of each), `$lazyChain` (`a`'s, then `b`'s),
  `$lazyTakeWhile`, `$lazyMapWhile`, `$lazyScan` and `$closing`.
- **`drop`'s (`skip`'s)** is its source's `nth(n - 1)`, then its source's
  `tryFold`. Its `nth(k)` is its source's `nth(n + k)`.
- **`filter`'s and `$lazySkipWhile`'s `next`** is its source's `find`. A
  `filter` needs no `tryFold` of its own: stepping that `find` asks its
  source what std's would.
- **`eq`'s other side** is `as_js_iterator`'s, as `chain`'s and `zip`'s are.
  `js_iterator` gives back what's a JS iterator already, where it wrapped
  it again.
- **`nth` is passed on as std's is** (#182). `take`'s `nth(k)` is its
  source's while it has more than `k` left. `$stepBy`'s `nth`, `fold` and
  `tryFold` take its first item by its source's `next`, then each by its
  `nth`. Where std's `nth` reaches the crate's own,
  `Recognition::nths_its_own` (of it, through a `&mut`, a `skip`, a `take`
  and a `step_by`), `it.nth(k)` is `$nth(it, k)`, as `it.next()` is
  `$next(it)`, where it was `it.drop(k).take(1)`, a `skip`'s `next`. An
  `enumerate` of what reaches it is `$enumerate`, whose `nth(k)` is its
  source's, counted `k` more, and whose `fold` and `try_fold` are its
  source's, where it was `map((x, i) => [i, x])`.
- **What takes by `fold`** is given `$folded(it)` where its fold asks an
  `nth` that's the crate's own, `folds_by_its_own_nth` (a `skip`'s and a
  `step_by`'s, through each adapter whose fold is its source's), as where
  it reaches the crate's own `fold`: `last`, and `collect` into a `String`
  or a set. Of a type parameter's, which may be the crate's own, too:
  `$folded` of a JS iterator with no `fold`, an array's, is itself. Both
  questions walk one list, `folded_sources`.

## Consequences

- Held by `test/corpus/skip_try_fold.rs` and
  `test/corpus/own_nth_through_adapters.rs`, against native Rust, and by the
  mutations of each file changed.
- These still step by `next` (#182):
  - a `peekable()` and a `fuse()`, std's bodies compiled (ADR 0561), whose
    dictionary has no `try_fold`;
  - `flatten` and a `flat_map` of what isn't a `$Steps`, which are JS's
    own `flatMap`;
  - and std's `nth` of a `fuse` and a `peekable`, which pass it to their
    source.

  Each differs only where an iterator writes its own `nth`.

## Amendment: `nth` through a `fuse` and a `peekable`, and `last` passed on

The rest of #182: a `fuse()` and a `peekable()` stepped the crate's
iterator by `next` where std's `nth` asks its own, and so did a `skip`
behind them, by its `nth(n - 1)`; and std passes `last` on, which
rust-js folded: a `skip`'s, after its `nth(n - 1)`, a `fuse`'s, a
`peekable`'s, a `copied`'s, a `cloned`'s, and a chain's, each side's.

- **`Fuse` and `Peekable` compile std's `nth` and `last`** (ADR 0561),
  which their JS iterator has, `$stdIterator`'s `nth` and `last`, made by
  `user_method`, the iterator's function of what else the method takes.
- **The crate's own `last` is `$iterator`'s too**, as its `fold` and
  `nth` are (`OwnMethods`), and its dictionary's; `$Steps` has std's
  default, of its `fold`; a `skip`'s asks its source's `nth(n - 1)`, then
  its `last`; a chain's asks both sides', and gives the second's where it
  has one. `lasts_its_own` answers where `it.last()` is `$last(it)`.
- **What passes items with a destructor uses an own method only where
  it's one**: `ownNth` and `ownLast`, which `$iterator` and `$stdIterator`
  set, say an `nth` or a `last` drops what it passes itself;
  `$nthDropping` and `$lastDropping` ask it then, and step and drop each
  otherwise, where std's defaults of a `$Steps` would drop none.
- **Still apart**: `collect()` into a `Vec` doesn't ask the crate's own
  `size_hint`, which std's asks as its capacity runs out, how often its
  growth decides, which no program is written for; listed as the guide's
  difference 21.
- Held by `test/corpus/own_last_and_nth_passed_on.rs` and
  `test/corpus/own_nth_last_dropping.rs` against native Rust, and the
  mutations `nths-its-own-no-fuse`, `lasts-its-own-not-through-skip`,
  `lasts-its-own-not-through-chain`, `skip-last-not-nth`,
  `chain-last-first-unasked`, `std-iterator-nth-unhooked`,
  `std-iterator-last-not-own`, `nth-dropping-default-nth`,
  `last-dropping-own-unused` and `compiled-last-unhooked`.

## Amendment: through a `Take`

A `Take`'s `fold` is std's `try_fold` of its source, and its `count`
advances its source, `advance_by`, which steps the crate's iterator by
`next`. rust-js folded a `Take` by its `next`, so `skip(1).take(3).sum()`
asked the crate's own `nth(1)`, where std asks `nth(0)`, and
`skip(2).take(2).count()` asked `nth(2)`, where std asks none (#213). A
`Take`'s `$Steps` fold is its `tryFold`, its source dropped after;
`advanceBy` is std's of a `Take` and a `Skip`; `count()` of a `Take` over
what reaches the crate's own `nth` is `$count(it)`, its own `count`; and
a `Take`'s `fold` is its source's to `folded_sources`, so `last()`
reaches it too (`skip_take_own_nth`, six mutations).

## Amendment: std's defaults of an adapter, and adapters given to std's code

Generated programs of adapter pairs over an iterator writing its own `nth`
and `fold` found 172 of 1,728 asking them otherwise than std (#214).
Four of std's answers were missing:
- a `skip_while`'s `fold` is its first step, then its source's own fold;
- a `$Steps`' default `nth(k)` is std's, `advance_by(k)` then `next`,
  and `advance_by` std's default of a sized iterator, its `try_fold`, so
  `step_by(2).map(..).nth(1)` reaches the `StepBy`'s `try_fold`, first by
  `next`, then `nth(1)`;
- std's adapter over the crate's iterator given to std's compiled code,
  a `fuse()`'s source, is a dictionary of `next` and, where its chain
  reaches the crate's own, `$foldOf`, `$nth` and `$last`, so the
  `Fuse`'s `nth` reaches the `Skip`'s.
`adapters_own_nth_fold` holds them, with four mutations. 40 programs
still differ, which #214 lists.
