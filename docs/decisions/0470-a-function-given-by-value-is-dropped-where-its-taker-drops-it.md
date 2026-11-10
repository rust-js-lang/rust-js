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

## Amendment: step 1b, each fold that moves a function drops it first (#734, #754)

After step 1, the iterator tables still listed 197 of the matrix's cases
and 953 of the conformance table's under #734, and #754's rfolds, where a
closure a fold by value moves drops after what's left, or never. Four
causes, each fixed where its owner is; the results change, so Rust's
behavior is kept (ADR 0262):

1. **A consumer over what reaches the crate's own `fold`.**
   `consumed_function` left out an iterator whose fold may reach the
   crate's own, `own.take(3).fold(0, f)` say, as that fold drops what
   it's given itself; but `fold_entry` gives the crate's fold `ends` as
   its `F`'s drop, so `$consume` drops it once there too. The exclusion
   is gone.
2. **A held function's fold of a source closed at its end.** `$holding`'s
   fold, an adapter's closure moved into the fold it asks, didn't hold
   what it steps, as `$consume` does: a `$closing` source closed itself
   as it was stepped to its end, a `step_by()`'s by its `nth`, before
   `ends`. It's held now, so what it steps closes as it's closed, after
   the function. Each part is still closed in turn as it's done with
   (ADR 1092's S9): a `$lazyChain` closes its first side itself.
3. **A held function folded from the back.** `$holding` had an `rfold`
   only where what it holds had one, and a `map()` of a `Vec`'s items,
   `$mapEnds` of a JS iterator, has none: it was stepped by its back,
   its function dropped as it was closed. It's `$foldBack` of its back
   now, moved as its fold is, as std's `Map::rfold` moves its function
   into its source's. And `$filterEnds`' `rfold` passes `ends` on.
4. **A call compiled into `@rust-js/std` but lowered as std's method.**
   `Rev::rfold` resolves to its body compiled into `@rust-js/std`, so
   dispatch took it for a call whose callee drops the function it's
   given; but the build that rewrites adapters lowers it as std's
   iterator method, `$foldBack`, which drops nothing:
   `(1..6u32).rev().rfold(0, f)` leaked `f`'s captures with no panic
   (#754). `mir_call_values_given` asks `std_iterator_call`, the answer
   the call's lowering asks, too.

`test/corpus/folded_functions_drop_before_the_rest.rs` checks each, and a
chain under a map that holds a closure, against native Rust in both
builds. The mutations are `consumer-own-fold-excluded`,
`holding-fold-unheld`, `holding-rfold-of-its-back-missing`,
`filter-ends-rfold-ends-lost` and `std-method-call-unasked`; step 1's
`enumerate-fold-ends-lost` and `fold-back-ends-after`, which no shape
reached then, are restored on the case. The tables' known list takes off
1131 cases; #754 has none left.

What the tables still list of it moves to the issues that own it, and
#734 has none left: a `chain`'s first side whose fold panics, closed after
the closure where std drops it first (#757, the one-`Chain` work); a
`skip_while`'s predicate that panics before it reaches its source's fold,
where std's `SkipWhile::fold` and `try_fold` drop their `fold` before
`init`, their parameters, and `try_fold` takes no `ends` (#768); and a
map's function as a drop panics (#762).

## Amendment: step 2, `max_by` and its kind (#734)

`max_by`, `min_by`, `max_by_key` and `min_by_key` of the crate's iterator
with a destructor were refused, "`max_by` of a value with a destructor":
they were std calls only the recognizers wrote, which a whole iterator
that drops something they don't close. They're std's iterator methods
from MIR now, beside `max` and `min` (`IterMethod::MaxBy`, `MaxByKey`),
through one builder of their helper's call, `extreme_by_call`, which the
recognizers' arm asks too. Std writes each as a `reduce`, which moves its
function into its fold: `consumed_function` says so of them, and their
helper is given what drops with it, `ends`, which `$extremeBy` runs as
`$consume`'s `reduce` does, before what's left. A `_key` form is std's
`self.map(key).reduce(..)`: where `reduce`'s first step ends, or throws,
the `Map` drops whole, its source and then its key, so `$extremeByKey`
runs `ends` after what it steps there, before it once folding began.

A key with a destructor over the crate's iterator, whose drop its caller
runs after the call, is refused, "`max_by_key` of a key function with a
destructor over the crate's iterator with one": where its first step
ends, std drops the key after the iterator, which `ends` can't, run
within the call. That's valid Rust, refused; left to do (#776).

`test/corpus/max_by_drops_its_function_first.rs` checks each against
native Rust in both builds: a closure that drops over the crate's
iterator, a plain one over it, one over a range and a `Vec`'s, empty, as
a panic in it unwinds and of items with a destructor. The mutations are
`extreme-by-ends-lost`, `max-by-not-from-mir`, `max-by-unconsumed`,
`extreme-by-ends-unrun` and `extreme-by-key-ends-before-its-source`.
`eq_by`, `cmp_by` and `partial_cmp_by` are unstable, `iter_order_by`, so
no stable program calls them.

## Amendment: a key with a destructor, and what `max_by` keeps as a later drop panics (2026-11-05, #776, #785)

Case ([0262](0262-when-rust-and-js-disagree.md)): results change, so
Rust's behavior is kept. A key with a destructor was a refusal (#776),
and the kept value's drop was a wrong answer (#785, P0).

**A key with a destructor** (#776). `max_by_key(f)` is std's
`self.map(key(f)).max_by(compare)`, of `(f(&x), x)` pairs. Then:
- `cmp::max_by(v1, v2)` returns one pair and drops the other, as a tuple
  drops: its key, then its item, the item though the key's drop panics;
- `let (_, x) = ..?` drops the kept pair's key before `Some(x)` is
  returned. If that drop panics, `x` drops as it unwinds.

`extreme_by_call` gave up on such a key with "a key with a destructor". It
now passes the key's drop last, so no other call changes:
`$maxByKey(items, key, compare, boxed, drop, ends, dropKey)`.
`$extremeByKey` drops each pair it passes over by `$inTurn`, key then item,
and drops the kept pair's key before it gives the item back.

**What's kept, where the one passed over panics as it drops** (#785).
`cmp::min_by(v1, v2)` keeps `v1` and returns it while `v2` drops. Where
`v2`'s drop panics, rustc leaks what the function was returning, as ADR
0465's #648 amendment says of a later drop that unwinds. `$extremeBy`'s
`keep` left `holding` set, so its catch dropped the kept one as it
unwound: `vec![D(12), D(13), D(14)].into_iter().min_by(..)` dropped `D(12)`
where D(13)'s drop panicked. `keep` now clears `holding` where the drop of
the one it passes over throws. That is so of `max_by`, `min_by` and their
`_key` forms, whose pairs drop the same way.

Still refused: a key *function* with a destructor over the crate's
iterator (above, step 2). Where `reduce`'s first step ends, std drops the
`Map` whole, the crate's iterator before the key function. In JS the
caller drops the crate's iterator after the call, and `ends` runs within
it. Making that order std's needs the consumer to own the iterator's drop,
which is #734's consume primitive.

`test/corpus/keys_with_a_destructor_drop_where_std_drops_them.rs` checks
`max_by_key` and `min_by_key` of keys with a destructor against native
Rust in both builds:
- over the crate's iterator, a slice's, and a `Vec`'s items with a
  destructor;
- empty and of one item;
- a key's function, the keys' comparison, a passed-over key's drop and
  the kept key's drop, each panicking;
- `min_by` and `max_by` where what they pass over panics as it drops.

Mutations: `max-by-key-drop-unpassed` (`lower/iterators/ends.ts`), and
`extreme-by-key-pair-key-undropped`, `extreme-by-key-pair-not-in-turn`,
`extreme-by-key-kept-key-undropped`, `extreme-by-key-kept-item-leaked` and
`extreme-passed-drop-panic-drops-kept` (`runtime/max_by.ts`).
`max-by-drops-none` and `extreme-by-listed` name their lines as they now
read. Kind: missing case, both. Area: iterators.

## Amended (2026-11-06, #789): a dropping key over the crate's iterator, closed

Step 2's refusal, "`max_by_key` of a key function with a destructor over
the crate's iterator with one", is gone. Std's `max_by_key` is
`self.map(key).reduce(..)`: its fold drops the key, then what's left of the
iterator, but where `reduce`'s first `next` gives nothing, or the key it
calls panics, the `Map` drops whole, the iterator, then the key. The
crate's iterator was dropped by its caller after the call, so the empty
case's order couldn't be std's. **The iterator is given to `$maxByKey`
closed, `$closing($iterator(it, next), drop)`**, as a `Vec`'s source is:
`$consume` drops the key, then closes it, at the fold's end, and
`$extremeByKey`'s `after` runs the key's drop after it closes, where the
first step ends. One answer, `keyed_consumer_closes` (`mir/iter/collect.rs`,
beside `consumed_function`, which it asks): `extreme_called` closes the iterator, and
`dropped_after` drops nothing after the call. The user's decision, through
m6-react: an existing form, no other changed.

Case ([0262](0262-when-rust-and-js-disagree.md)): results change, so
Rust's order is kept; a refusal of valid Rust lifted. Held by corpus
`max_by_key_of_a_dropping_key_over_its_iterator`, against native Rust in
both builds: `max_by_key` and `min_by_key`, empty and not, a key that
panics on the first item and on a later one. Mutations
`keyed-extreme-unclosed` (`mir/iter/collect.ts`) and
`keyed-extreme-dropped-after` (`mir/iter.ts`). The guide's limitation is
gone.

## Amendment: given through another bound (#966)

A closure called by `&` that holds a value with a destructor, given for
an `impl Is<F>` whose `F: FnOnce()` the callee calls once, reached `F`
through `Is`'s method, which `check_called_once` didn't see: its captures
were never dropped. A parameter whose own bound takes such an `F` as it
is, an argument of a trait not `Fn`'s, is refused as `F` itself is. Corpus
case `fn_once_through_another_bound_refused`; supporting it, an `Fn`
closure's drop after a generic `FnOnce` call, is #995.

## Amendment: a library's function the crate compiles itself (#1051)

Case ([0262](0262-when-rust-and-js-disagree.md)): a refusal lifted, with
no wrong answer before it. Another crate's function given a closure that
holds a value with a destructor was refused. That includes a library's
generic function the crate compiles itself, because it's given closures
that drop (ADR 0914). The same program compiles in one crate:
`fold_with(1..4, move |x| .., 0, move |s, x| ..)` of a closure that
holds a `Loud`. Such an instance is the crate's own function. It takes
each drop its caller gives, `dropF?.(f)`, as the crate's own does, so
`drops_given` answers for it as for the crate's own
(`foreign.is_instance`). Another crate's JS stays refused as before.

What `fold_with` gives its closures to must take their drops too.
`joined(f, g)` makes `move |acc, x| g(acc, f(x))` and gives it back,
and the closure carries its drop (ADR 0484). Its library's JS takes no
drop, so its closure carried none, and `fold`'s end dropped neither. So
a library function that makes a closure of what holds its type
parameter may drop it (`body_may_drop`'s `closes_over`), and the crate
compiles its own of it where it gives it a value that drops. It's
compiled as the crate's own `joined` is, `joined(f, g, dropFnMut,
dropFnMut$1)`, its closure `$dropping`.

What an instance makes of a closure it's given is the crate's to see, as
the crate's own function's is. A `dyn Fn` it makes, `node`'s
`Box::new(run)` of its `F`, is one of the crate's `dyn Fn`s that may drop
(ADR 0626, amended). Its MIR's coercions are read where it has no THIR
(`fn_dyn_sources`). Where the library's `dyn Fn`s drop nothing, the
crate is refused as ADR 0626 refuses a crate's own: "this crate's `dyn
Fn`s may hold a value with a destructor, which their drop runs, but
`app`, a rust-js library whose `dyn Fn`s drop nothing, holds one of
them", naming `node`'s coercion. An integration test that gave a
program's `node` such a closure was refused as given away. Without this
it compiled, and its `dyn Fn` dropped nothing.

The pairs `closure_made_by_an_inlined_function`, moved from
`test/crates/refused/` to `test/crates/pairs/`, and
`instance_makes_a_dyn_of_what_drops` run as native. The refusal is
`test/rust-js-test-programs.test.ts`'s "boxes as one that drops nothing",
its message now ADR 0626's. The mutations are
`instance-given-closures-another-crates` (`lower/drops/facts.ts`),
`closure-of-a-parameter-drops-nothing` (`lower/instances.ts`) and
`instance-dyn-fns-unseen` (`lower/analysis/closures.ts`).

## Amended (2026-11-10, #1220): a boxed function is called as its taker calls it

A `Box<dyn FnMut(..)>` given by value to `all`, `any`, `fold` or
`for_each` was never dropped: not after the call, and not as a panic in it
unwound. Native Rust drops it as the call returns. How a given function is
called decides where it's dropped, and that was read from the given type
alone (`function_kind`): a closure's kind, or a type parameter's bound. A
box has neither, so it was taken for a value std keeps. A `FnOnce` box
given to `or_insert_with` or `catch_unwind` was right only because those
list it as unused where it isn't run.

Where the given type doesn't say, its callee's own parameter does,
`any<P: FnMut(Self::Item) -> bool>`'s `P` (`given_unused`'s `called`).
A box `push(T)` keeps, whose `T` no `Fn` bound names, is still kept, never
dropped after the call. Case C ([0262](0262-when-rust-and-js-disagree.md)):
a destructor's output is seen.
`test/corpus/boxed_fn_given_to_consumers_drops.rs` holds each consumer
and a panic unwinding through `any`, against native Rust.

Reopened the same day by #1204's context rewrite, `at_0(Box::new(f)) as
Box<dyn ..>`, two more of the same root. `or_insert_with` of a key that's
there leaves its function unrun and drops it there (ADR 0469), which
`drops_unrun` asked of the given type's kind alone; a box, local or made
through a call, now counts as one its taker calls once. And std's compiled
`Scan` holds its function through an arrow that gives it the state's box,
`(x0, x1) => f(x0.value, x1)` (ADR 0454), and drops it by `$dispose`, which
the arrow carried nothing for; the arrow of a boxed `dyn Fn` that drops
carries the box's drop now, `$dropping(arrow, () => $dispose(f))`.
`test/corpus/boxed_fn_through_a_call_drops.rs` holds both, in both builds.
