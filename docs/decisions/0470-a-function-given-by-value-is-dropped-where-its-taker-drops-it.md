# 0470. A function given by value is dropped where its taker drops it

Status: Accepted. Extends [0469](0469-a-function-std-leaves-unrun-is-dropped-there.md),
[0098](0098-destructors.md) and [0190](0190-generic-no-destructor.md).

Case: C ([0262](0262-when-rust-and-js-disagree.md)): what's dropped, and
when, as std's.

## Context

A closure that holds a value with a destructor and is called by `&` or
`&mut`, `move |x| x < limit.get()`, never drops it as its body ends: what
holds the closure does, once it's done with it. Given by value, to std's
`all`, `fold` or `sort_by`, or to a generic function's `F: FnMut`, it was
refused (ADR 0098), as JS saw no one drop it. itertools gives its own
`move |elt| used.insert(elt)` to `all`, and `move |a, b| cmp(b, a)` to its
`k_smallest_by`.

## Decision

**Who drops a function given by value is decided by how it's called**,
`recognition::function_kind`: a closure's kind, or a type parameter's
strongest `Fn` bound.

- **One called by `FnOnce` alone** drops what it holds as its body ends;
  where it's left unrun, std drops it there (ADR 0469).
- **One called by `&` or `&mut`, given to std**, is dropped as the call
  returns, as std's end drops it: `all`, `fold`, `sort_by`, and an
  `Option`'s `map` of one, run or not. The wrapper around every std call,
  `mir_call_values`, holds it and drops it after:

  ```js
  const all = $lent(tmp).every((item) => value(item));
  loudDrop_drop(a);
  ```

- **Given to the crate's own function**, of an `F: FnMut` or `F: Fn`, it's
  that function's to drop, as its caller's drop says, `dropF?.(f)`. Given
  where it's called by `FnOnce` alone, `G: FnOnce`, it's refused: that call
  drops none of it, where Rust's drops it once it's run.
- **One what it's given to holds**, a lazy adapter's, `map(f)`, is refused,
  till an adapter drops its function as it's closed.
- **A binding's**, which JS holds, and another crate's, stay refused.

## Why

- **It's std's**: `given_function_drops` runs `all`, `fold`, `sort_by`, a
  generic `F: Fn` and `F: FnMut`, and a closure called before it's given,
  with native Rust.
- **One place answers each question**: what kind a function is
  (`function_kind`), where std drops one (`handled_function`), and what
  `check_takes_drops` leaves to it.

## Consequences

- **What compiles grows**: a closure that holds a value with a destructor,
  given by value to std or to a generic function, as itertools' are.

## Amendment (2026-11-05): and as the call unwinds (#719)

Case ([0262](0262-when-rust-and-js-disagree.md)): results change. A
function called by `&` or `&mut` that std is given by value was dropped
after the call returned, and never where the call panicked: a
`for_each`'s, a `fold`'s, an `any`'s, a `position`'s, a `retain`'s or a
`sort_by_key`'s closure whose own call panics leaked what it holds, and
so did, under `--std-iterators`, a `map`'s that std's compiled `fold`
moved into its source's (#710). Rust drops it as the panic unwinds: std's
call owns it, and its frame drops it.

**`mir_call_values_given` drops what it drops as the call returns as a
panic in the call unwinds too**: the call made inside `unwinding`'s `try`,
`catch (error) { loudDrop_drop(a); throw error; }`, which a body's tidy
pass reads as the `finally` of the drop after. Only what's dropped
whatever the call gives, a function called by `&` or `&mut`; not one std
may leave unrun or unused (ADRs 0469, 0490), whose drop is a variant's,
nor one called by `FnOnce`, whose body drops what it took. A drop that
std's compiled fold already ran as it unwound, given as its `ends`, runs
no more: `$dispose` and a `$holding`'s drop are once (ADR 0991, amended).

`test/corpus/closure_folded_by_value_panics.rs` checks each against native
Rust, as the crate builds it and with std's iterator structs. The
mutation is `given-undropped-as-it-unwinds` (`lower/mir/dispatch.ts`).

## Amendment (2026-11-05): a by-value fold drops it first, `$consume` (#734)

Case ([0262](0262-when-rust-and-js-disagree.md)): results change, with no
panic. `It(3).fold(0, move |a, x| { let _g = &g; a + x })`, of the
crate's iterator `It` with a `Drop`, printed "drop it, drop f", where
Rust prints "drop f, drop it". The same held through each adapter whose
fold reaches it, a `skip`'s, `map`'s, `take`'s, `step_by`'s, `chain`'s,
`enumerate`'s, `peekable`'s, `fuse`'s, `filter`'s, `zip`'s, a `rev()`'s,
and of `for_each`, `rfold` and `reduce`. `fold(self, init, f)` drops `f`,
the closure, before what's left of `self`, at its end and as it unwinds
(iterator.rs:2660). The lowering dropped the closure after the call
(`dropped_given`), outside the iterator's own drop and what its fold
closed.

**A by-value consumer's own function, where its drop is seen, is moved
into the fold: `$consume(it, step, init, f, unwind, ends)`.**

- `ends`, the function's drop, is given to the fold `it` runs, which drops
  it before what it has left (ADR 0991's protocol), then dropped by
  `$consume` once, then `it` is closed, at its end and as it unwinds.
  `step` is "fold", "rfold" or "reduce" (whose `init` says its `Some` is
  boxed); `for_each` is a fold. The iterator's own drop stays its
  caller's, `dropped_consumed`'s, after the call, under its guards: **the
  user's decision**, the visible form `try { return $consume($iterator(
  iter, itIterator_next), "fold", 0, f, undefined, () => {
  dDrop_drop(g); }); } finally { dDrop_drop(iter._held); }`, its `rest`
  in the outer `finally`, where the first proposal had it inside
  `$consume`.
- `consumed_function` (`mir/iter/collect.rs`) alone says which:
  `fold`'s, `rfold`'s, `for_each`'s and `reduce`'s last argument, of a
  function whose drop drops something, over an iterator that drops
  something after it or whose fold moves a function an adapter holds; and
  in std's compiled body, the `F` it was given or a closure that carries
  its drop, as #710's `ends` was. `consumed_ends` makes its drop, and
  `dropped_given` skips it. **The user's decision**: elsewhere the form is
  as it was, `.reduce(f, 0)`, `.forEach(f)`, where nothing it drops is
  seen in its order. Not a `&mut`'s, which `try_fold`s, nor where the
  crate's own `fold` is reached, which drops what it's given itself.
- What `$consume` folds is held (`hold()`), so it closes as `$consume`
  closes it, after `ends`, never as it's stepped to its end, a `step_by`'s
  or a `take`'s source's: std's fold drops `self` whole after `f`.
- Each runtime fold and `rfold` that takes an `unwind` takes `ends` too,
  and passes it on where std passes its `f` by value: `$lazyChain`'s to its
  second side alone, as `Chain::fold` gives its first `&mut f`, and
  `$chainEnds`' `rfold` to its first; `$stepBy`'s, `$enumerate`'s,
  `$lazySkipWhile`'s, `$lazyScan`'s, `$skipping`'s and `$taking`'s
  `rfold`, `$foldBack` and `$iterator`'s and `$foldOf`'s entries. A
  flatten's drops it after its parts, as `FlattenCompat::fold` folds them
  by `&mut fold`. An architecture test refuses a runtime fold that takes
  `(init, f, unwind)` alone.

`test/corpus/consumer_closure_dropped_first.rs` checks each consumer and
adapter above, a `Vec`'s rest, a flatten, a `by_ref()`, and a panic in
the closure, against native Rust, as the crate builds it and with std's
iterator structs; `iterator-forms.test.ts`' "a consumer's closure folded
over a reversed adapter", a `skip()`, `chain()` and `filter()` reversed,
which the build that rewrites adapters refuses. Architecture tests:
"every runtime fold takes what drops with its f" and "a consumer's
closure folded by value is dropped by $consume alone". The mutations are
`consume-unheld`, `chain-fold-ends-lost`, `step-by-fold-ends-after`,
`skip-while-fold-ends-lost`, `iterator-rfold-ends-lost`,
`consumed-function-never`, `consumed-rfold-not-by-value` and
`consumed-function-dropped-after`.
