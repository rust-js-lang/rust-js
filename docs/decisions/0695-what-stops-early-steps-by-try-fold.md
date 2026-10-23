# 0695. What stops early steps by `try_fold`, and `nth` is passed on, as std's adapters route them

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
