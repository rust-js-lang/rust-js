# 0991. Every adapter that holds a function drops it

Status: Accepted. Extends [0485](0485-an-adapter-holding-a-function-is-closed-to-drop-it.md)
from `map` and `filter` to each adapter that holds a function. Found by
#382's triage of `inspect-holds-no-function`.

Case ([0262](0262-when-rust-and-js-disagree.md)): results change. What a
closure held was never dropped where Rust drops it, and four forms were
valid Rust refused.

## Context

```rust
let n = Noisy(7);
let seen = v.iter().inspect(move |x| { let k = &n; println!("{x} {}", k.0); }).count();
```

Rust drops the `Inspect`, and the closure with `n`, as the statement ends:
`drop 7`. ADR 0485 made a `map` or a `filter` whose function holds a value
with a destructor `$closing(it, (it) => { it.return(); drop f })`, in
`mir_iter_method`'s `holding_function`. `held_function` also named
`inspect`'s and `skip_while`'s, so the refusal of a kept function let them
through. But these are combinators, which `std_calls` lowers, and nothing
there dropped the function: `drop 7` was never printed. `flat_map`'s
function wasn't refused either, and wasn't dropped. `filter_map`,
`take_while` and `map_while` were refused.

## Decision

`held_function`, the one list of what an adapter holds as its function,
names each adapter's: `map`, `filter`, `filter_map`, `take_while`,
`skip_while`, `map_while`, `inspect` and `flat_map` (whose `F` follows
what it flattens). `std_calls`' combinators close an adapter that holds
one with a destructor as `holding_function` does a map's. Both use
`closing_function`: its source is closed, then its function is dropped,
as Rust drops an adapter's fields.

`scan()` holds its state too, so one whose function holds a value with a
destructor stays refused. The diagnostics test "a function with a
destructor a lazy take_while holds" pinned the old refusal, and is
removed.

## Consequences

`test/corpus/adapter_functions_dropped.rs` checks each adapter against
native Rust, consumed to its end, stepped part of the way, and held to its
scope's end. The mutations are `combinator-function-undropped` and
`flat-map-function-unheld`; `inspect-holds-no-function` and
`held-function-unclosed` point at the case too.

The same commit removes `Rev` from `back_stepped_source`, which nothing
reached once a `rev()` made in place is read by its form (ADR 0987). The
corpus, the forms corpus, the snapshots and memchr's shipped test are
unchanged without it. Its mutation, `rev-not-back-stepped`, is removed
(#382).

## Amendment (2026-11-05): where a fold drops it (#710)

Case ([0262](0262-when-rust-and-js-disagree.md)): results change.
`rest.into_iter().take(2).map(move |d| { let _g = &guard; d }).chain(ds())
.collect::<Vec<_>>()` dropped `guard` after what the `take` left, 3, 4
and 5, where Rust drops it before them, with no panic. A `sum()`, a
`count()`, a `fold`, a `max()`, an `extend`, a `collect()` into a
`String` or a set, and a `TrustedLen` collect into a `Vec` were the same.
`$closing(adapter, (it) => { it.return(); drop f })` drops the
function after its source, as Rust drops a `Map`'s fields. But a fold by
value doesn't drop the `Map`: `Map::fold` moves its `f` into the closure it
gives its source's fold, `self.iter.fold(init, map_fold(self.f, g))`, and
`fold(self, init, f)` drops `f` before what's left of `self`, a `Take`'s
rest, at its end and as it unwinds.

**An adapter that holds a function with a destructor is `$holding(adapter,
() => drop(f), moved)`. Its function drops, once, where std's fold drops
it, and otherwise after what it steps, as before.**

- A JS fold takes `ends`, what drops with its `f`, as a fourth argument.
  Each fold that closes what it has left calls `ends` first, at its end and
  as it unwinds (`$closedAfter`): `$Steps`' own fold, a `take`'s, a
  `skip`'s, `$skipping`'s, `$closing`'s, a `take_while`'s and a
  `map_while`'s. An adapter that passes its fold on to its source's
  passes `ends` on too (`$forward` passes every argument), as std's
  passes the closure that holds it. A fold that doesn't take it drops
  nothing early, so what it closes drops as it did.
- `held_function_folded`, beside `held_function` in one catalog
  (`recognition::iterators`), says where each adapter's fold drops its
  function. `map`'s is `"after"`: it's moved into the fold of what it
  steps, after the `ends` it's given, as `map_fold(f, g)` drops `g` first.
  `filter`'s, `filter_map`'s and `inspect`'s are `"before"`: their closures
  name their own function first. `flat_map`'s is `"alone"`: its function
  goes into its source's fold alone, and the `ends` it's given drops after,
  as `FlattenCompat::fold` folds its `Map` by `&mut fold`.
  `take_while`'s, `map_while`'s and `skip_while`'s are kept: their fold is
  `try_fold` by `&mut self`, or keeps the predicate in `self`, so the
  function drops with the adapter, after what it steps.
- A fold std takes by `&mut`, an in-place collect's `try_fold`, moves no
  function: `$foldToArray` in place steps it by `tryFold`, then closes
  it, so the function drops after what's left, as
  `forget_allocation_drop_remaining` drops it before the iterator.
- Where std folds and rust-js stepped by `next`, a consumer whose fold
  may reach such a function (`folds_held_function`, through the
  `OwnStep::Fold` catalog) folds now. A `TrustedLen` `collect()` into a
  `Vec` not collected in place is `$foldToArray`, as `extend_trusted` is.
  A `Vec`'s or a deque's `extend` of one is `$extend(v, it, true)`. A
  `step_by(n)` over one is std's `$stepBy`, which steps it by `nth`, where
  a filter's fold would move the function.
- Std's compiled structs, a `peekable()`'s or a `fuse()`'s source in the
  default build and each adapter under `--std-iterators`, drop it where
  their MIR does. A compiled fold is given `ends` as its own `F`'s drop
  (`fold_entry`), which its body drops before what it has left, as
  `fold(self, init, f)` drops `f`. A fold it asks of a JS iterator is given
  `ends` in turn (`folded`): the drop of the closure it moves into it,
  `Map::fold`'s `map_fold(f, g)`, which carries its drop (`$dropping`), or
  of std's body, the `F` it was given, whose drop is its caller's `ends`.
  The drop after the call
  stays, and drops nothing again: `$dispose` runs a closure's drop once,
  as Rust drops a value once, and `ends` is once. A collect in place of
  items with nothing to drop is said so by its caller (`$foldToArray`'s
  `inPlace`), where a compiled `Map`'s fold would move its function.
- Apart: a call std is given a closure by value, of std's compiled body,
  drops it only as the call returns, so a panic in a compiled `Map`'s
  closure leaks it under `--std-iterators`, where Rust drops it as it
  unwinds (`test/corpus/closure_folded_by_value_panics.rs`, filed apart).

`test/corpus/fold_drops_moved_function_first.rs` checks each consumer and
adapter above against native Rust: by value, by `next`, by `&mut`, in
place, two closures, a panic in what's left, as the crate builds it and
with std's iterator structs (`test/iterator-forms.test.ts`);
`closure_folded_by_value_panics.rs`, a panic in the closure, as the crate
builds it. The
mutations are `holding-moved-ignored`, `holding-before-after-swapped`,
and `holding-alone-as-after`
(`runtime/closing.ts`), `take-fold-closes-at-its-limit`
(`runtime/steps.ts`), `flat-map-own-unpassed` (`runtime/flat_map.ts`),
`extend-by-next` (`runtime/extend.ts`), `map-folded-before`,
`filter-folded-after`, `flat-map-folded-after` and `take-while-folded`
(`lower/recognition/iterators.ts`), `holding-unmoved`
(`lower/mir/iter.ts`), `folds-held-function-never`
(`lower/mir/iter/closing.ts`), `collect-trusted-by-next`
(`lower/mir/iter/collect.ts`) and `extend-trusted-by-next`
(`lower/combinators.ts`); of std's compiled structs, `fold-given-no-ends`,
`fold-given-no-ends-of-a-param` and `in-place-of-numbers-unsaid`
(`lower/mir/iter/collect.ts`), `compiled-fold-f-undropped`
(`lower/iterators/knowing.ts`), `std-iterator-fold-ends-lost`
(`runtime/std_iterator.ts`), `in-place-by-fold` (`runtime/fold_of.ts`) and `dispose-twice` (`runtime/dropping.ts`).
