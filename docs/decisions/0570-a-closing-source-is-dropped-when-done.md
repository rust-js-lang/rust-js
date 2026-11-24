# 0570. A closing source is dropped when it's done

Status: Accepted. Amends [0465](0465-dropping-an-adapter-is-closing-it.md).

Case: B ([0262](0262-when-rust-and-js-disagree.md)): a non-fused adapter
stepped again after its `None` drops its source earlier than Rust, which
is rare and listed under Consequences.

## Context

An adapter of an iterator of the crate's that has a destructor,
`Counter { .. }.map(f).sum()`, holds its source in `$closing(it, drop)`,
which runs `drop` when the adapter is closed: `return()`, as JS calls it
when a consumer stops early, a `for` breaks, or the chain is dropped
(ADR 0465). A consumer that runs to the end, `sum()`, `collect()`,
`count()`, or a `for` that finishes, never calls `return()`: JS's iterator
helpers and `for...of` call it only on an early exit. So the source was
never dropped at all, and its `Drop` never ran, where Rust drops it as the
statement or the loop ends. A silent difference, not a refusal, found by
`test/corpus/adapter_source_drops.rs` against native Rust.

## Decision

`$closing` drops what it closes the first time its iterator reports
`done`, as well as on `return()`, once either way. A chain stepped to its
end is done with: Rust drops the temporary chain at the end of the
statement that consumed it, and the iterator a `for` took at the end of
the loop, which is where the last `next()` returns `None`. No lowering
changes: the one place that drops a closed source, `$closing`, now knows
both ways it's closed.

## Why

- **It's where Rust drops it**, as the program can observe: native Rust
  prints `drop sum` before `total 6`, and `drop for` after the last item
  and before what follows the loop.
- **One place.** The lowering already gives every such source its drop
  through `$closing`; making a consumer call `return()` at its end would
  have been a rewrite of each consumer's JS, and a second way to say
  "closed".

## Alternatives

- **Call `return()` after each consuming chain.** Each consumer's JS gets
  a `try`/`finally` or a statement after it, for a fact `$closing` can see
  itself.
- **Drop at the end of the enclosing scope**, as a drop flag does. The
  chain is a temporary with no binding to hang the flag on.

## Consequences

- A non-fused adapter stepped again after it returned `None`, by hand,
  `it.next(); it.next()` past the end, drops its source at the first
  `None`, where Rust keeps it until the adapter is dropped; the steps after
  still run on the same JS object. Only the time the `Drop` runs differs.

## What holds it

- `test/corpus/adapter_source_drops.rs`: `map(..).sum()`,
  `filter(..).collect()`, `map(..).count()` and a `for` over a `map`, each
  of a source with a printing `Drop`, against native Rust.
- Mutation `closing-not-dropped-when-done`.

## Amendment: no second close after a consumer

`v.into_iter().copied().collect()` of a source closed to drop what it
holds was also closed again after its consumer, `iter.return()`, a
second way to do what this ADR's `$closing` does as it's done: the
nightly's mutation `consumed-copied-undropped` survived, as nothing
changed but the JS. A `copied()` of such a source, consumed by value, is
closed only as it's done: `$closing(..).toArray()`.
`test/corpus/consumed_adapter_drops.rs` holds it against native Rust.

## Amendment: done however it's stepped, and by what ends early

An iterator of the crate's with a destructor was never dropped through
much of std's adapters (#225): of 2,100 generated programs, 226 never
ran its `Drop`, and 30 of a `fuse()` of it stepped from its back threw.
`$closing` knew it was done by its `next` alone. Now:

- **Done however it's stepped**: by its `next`, its `back`, its `nth` or
  its `nth_back`, as a `step_by` steps the crate's own `nth` and a back
  form steps its back. It passes `$knowing` on to what it closes, so a
  compiled `fuse()` closed to drop it is given its back and length.
- **What ends early is done with what it steps**, as `$Steps`' `take` at
  its limit already was: a `skip` or a `take` ended from its back, a
  `take`'s `try_fold` asked again at its limit (a `filter`'s step), and a
  `skip`'s or a `take`'s `rfold`, which consumes it.
- **What's taken is closed to drop it**: a `filter` stepped from its back
  closes its source; `rev()` and an `enumerate()` made knowing its back
  close what they're given, by its own type, so not through a `&mut`.
- **What a `&mut` lends is stepped as it**: `user_iterator_knowing` sees
  through `$lent(..)`, as through `$closing(..)` (ADR 0465), and gives a
  `&mut` of the crate's iterator its own methods as std's `&mut I`
  passes them on, not its folds, which take and drop it:
  `(&mut it).map(f).rev().sum()` was 0, not 90, a wrong value.

Of the 2,100, 216 more agree; 57 still differ. Of those, 25 are a
`fuse()`'s own `rfold` and `nth_back` (#224). Nine are a chain stepped to
its end by a consumer that borrows it, `nth` or `find` say, whose source
is dropped as it's done, not at the statement's end. That's #206's
question of when done means dropped. Two drops of a `filter(..).fuse()`
stepped from its back no longer reach the crate's `rfold`, as `rev()`
closing a compiled `fuse()` hides it from "a `Fuse` stepped to its end is
its source" (ADR 0561). #224's fix covers them too.
`test/corpus/own_iterator_dropped_through_adapters.rs` holds the rest,
with 17 mutations; `closing` leaves what's closed already as it is, so an
owning source `rev()` is given isn't closed twice. The readability
baseline's `while (true)` share grows by 0.1 in 10,000 lines: the case's
`rfold`, a `while let`.

## Amendment: dropped where its owner drops it

A chain its owner drops, a variable's or a temporary's, now drops what it
holds where Rust does: at the owner's drop, the scope's or the statement's
end, not as it's done or an adapter in it ends (#206). Of 1,000 fuzzed
programs, 21 differed only in when a drop ran, `vec![D(8), D(9), D(10),
D(11)].into_iter().rev().take_while(..).nth(0)` dropping 8, 9 and 10
before the statement printed. A held `take`, `take_while` or `map_while`
stepped to its end dropped what its source had left at that end, not at
its scope's end.

- **What the body drops is held**: the value a call gives a local that
  the body drops where it runs on, not only as a panic unwinds past it,
  of a type dropped by closing it (`dropped_by_closing`, the drop's own
  rule), is `$held(..)` (`holds_dropped_iterator`). One only moved, into
  a consumer that takes it, isn't: that consumer's end is its drop.
- **Held, it's dropped only as it's closed**: `hold()` passes down
  through each adapter to what it steps (`from`, or `iter`); held, a
  `$closing` isn't closed as it's done, nor a `take` at its limit, a
  `take_while` or a `map_while` at its end, a `skip` or a `take` ended from
  its back. Its owner's drop, `it.return()`, closes it.
- **A consumer that takes a chain closes it at its end**, `$Steps`'
  `fold` and `toArray()`, as Rust's drop what they take: what a held source
  in it has left too, a variable moved into it on one path.
- **What JS's own consumers take is still closed as it's done**, as a
  `take` at its limit, a `take_while`'s or `map_while`'s end: `collect()`
  into a `HashSet` is `new Set(..)`, which closes nothing. #225's closes of
  a `skip` or a `take` ended from its back, and of `$closing` done from its
  back, go: whatever steps those is a `$Steps` consumer, which closes at
  its end, or is refused.

A chain consumed by value where it's made, never held, is still closed as
it's done, which is where its consumer drops it. The guide's difference
that a restarting source's adapter drops what it holds at its first
`None` is gone: one stepped by hand past its `None` is held. All 2,100
droppable programs of #225's sweep agree, and the 1,100 and 2,044 of
#220's and #216's; of fuzz seeds 42300–42799, 17 differed, 7 do now, each
`vec::IntoIter` passing over items in bulk, which std drops as a slice,
front to back (filed apart).
`held_iterator_dropped_by_its_owner` and `fold_back_drops_what_it_takes`
hold it, with twelve mutations. The readability baseline's `$iter` share
grows by 0.7 in 10,000 lines and `while (true)`'s by 0.1: the new cases'
arrays stepped, and their `while let` loops.

## Amendment: every consumer drops what it has left, and what an owner holds is held

#206 left three gaps, found by fuzz seed 43991 and by a probe of 23
adapters × 11 consumers × 3 sources, then 11 prefixes that stop early
× 23 consumers:

- **A consumer that takes a chain closes it at its end, on every path**
  (#242). `$lazySkipWhile`'s `fold`, whose first step found none; `last`
  (`$lastDropping`), `max`/`min` (`$extremeBy`) and `reduce` of none, as
  std's `next()?` drops `self`; `Skip`'s fold when its `nth(n - 1)` finds
  none; `step_by`'s fold; and `cmp`/`partial_cmp` (`$iterCmp`), `b` then
  `a`, as `$iterEq` already did. Since #206 an un-held `take` closed only
  when asked past its limit, so `take(1).skip_while(..).count()` never
  dropped what was left.
- **An un-held `take` stopped at its limit by its `try_fold` closes then**,
  as its `next` does, unless what stopped it was its caller breaking on
  the item it gave, which the caller still processes. A `filter` that
  kept none, collected by JS's own `new Set(..)`, which closes nothing,
  now drops what is left. A held `take` asked past its end by `nth`
  closes nothing, as std's `Take::nth` doesn't.
- **What a value the body drops holds is held** (#243). A call's value
  moved into another call whose value the body drops, and whose type
  contains it, `Peekable<Take<..>>`'s, is held (`owned_where_run`), since
  that owner's drop glue closes it where Rust drops it. A held chain's
  second side isn't closed as it's done: std's `Chain` lets go of `a`
  when it ends, never `b`.
- **A chain dropped by closing isn't listed** (`collected_items`).
  `Array.from(it).map(f)` drained `it`, and its closing dropped what was
  left before `f` ran on any item. Rust runs `f` on each item as it
  comes, then drops the iterator.

`step_by(n).rev()` of the crate's iterator asks its `nth_back` again, as
std's `next_back` does. #231 had made it `$stepByEnds`, whose back
stepped one item at a time (#245).

All 759 + 253 probe programs agree but `take(1).fuse().any(..)` and
`.position(..)`. std's `Fuse` of a `FusedIterator` keeps its inner
iterator, and the compiled generic one clears it (#244). The droppable
sweep's 2,100 programs agree. Held by `test/corpus/consumer_drops_what_is_left.rs`,
28 mutations, and `test/lowering-generic-iterators.test.ts`'s bare generic
`collect`. A value moved into an adapter that is held itself isn't held
again, since the adapter's `hold()` holds it. The readability baseline's
`$iter` share grows by 1.5 and its renamed `x$1` share by 3.2 in 10,000
lines. Both come from the new case alone: 20 `$iter(three())` sources,
and its shadowing `let taken`s, renamed apart in the `try`s that drop each.

## Amendment: what's stepped by reference, or given, is held (#392)

An adapter a variable held, stepped to its end by `next()` and then moved
into `drop(zipped)`, dropped its `$closing` source at its first `None`,
before the code between the loop and the `drop` ran. #206's rule held
only what the body drops, not what it moves, since a consumer that takes
a chain drops it at its end, which is where it's done. A variable the
body steps by reference first can be done long before what takes it.
Two more of the same: a function's parameter it steps by `next()` and
drops at its end was never held, as only a call's value was, and a
generic body's iterator of a type it's given, `items.into_iter()` in a
`FromIterator`, wasn't held either, since its type said nothing of a
closing source. And `$intoIter` of a `$Steps` stepped it by JS's own
protocol, `Iterator.from`, which closes it at its end, so even a held
one was dropped at its `None`.

Case **Results change** of ADR 0262: Rust's order is kept.

- **A local the body steps by reference is held** (`owned_where_run`'s
  `lent`): a `&mut it` or `&it` of the local itself, whatever takes it
  after.
- **A parameter is held as a local is** (`held_parameters`), with
  `$held(it);` at the function's start, the function owning what it's
  given: of a type dropped by closing, or of a type parameter or an
  associated type bounded as an `Iterator`
  (`Recognition::bounded_iterator`), which may be one. Not std's struct,
  compiled, which steps what it holds as Rust does and has no `hold()`.
- **`$intoIter` gives a `$Steps` as it is**, as `$stepsOf` does.

Considered: holding the argument in the caller wherever a crate's
function takes it. The caller can't tell a consumer from a function that
runs on after its argument is done, and `collect()` into the crate's own
`FromIterator` reaches the same body through std. The callee owns what
it's given, and says so. Held by
`test/corpus/held_adapters_stepped_to_their_end.rs` and four mutations.
std's compiled flatten holds its inner iterator, which it drops itself.
The readability baseline's `while (true)` share grows 10.4 to 10.5 in
10,000 lines, the new case's alone: its `zipped`, moved after its loop,
is stepped in the `while (true)` a variable's `$live` flag has had since
ADR 0570.

## Amendment: an iterator its type says is one isn't made one again

Case ([0262](0262-when-rust-and-js-disagree.md)): C, results change:
what a held iterator's source has left is dropped where Rust drops it.

Generated program 12454000 printed `vec![D(34), D(35), D(36)].into_iter()
.rev().take_while(|d| d.0 < 36).step_by(id(2)).next()` with `drop 34` and
`drop 35` before `None`, where Rust drops them after the line. The call in
`step_by`'s argument put the `take_while` in a variable first, which
`is_js_iterator` couldn't tell is an iterator from how it's written, so the
`filter` made of it was given `Iterator.from(..)`. That returns a `$Steps`
as it is, but calls its `[Symbol.iterator]`, which makes it close as it's
done, as JS's own loops step it: the held temporary dropped what was left
at its end, not its statement's.

**A value `as_js_iterator` passes on as an iterator its type says it is,
one implementing `Iterator`, is marked `Known::Iterator`, and
`is_js_iterator` answers it before it reads how one is written.** A
collection's, `eq(other)`'s array, isn't marked: it's made one there. Five
`Iterator.from(..)`s of the corpus's snapshots are gone with it.
`step_by_of_a_call_holds_its_items` is the case.

Amended 2026-10-30: with it, `collected_items`' own refusal of a chain
dropped by closing (`dropped_by_closing(iter_ty)`) went: `collected` lists
only `Iterator.from(list)`, never a runtime helper's list, and a closing
chain is `$closing(..)`'s, a JS iterator nothing wraps again. So the rule
that a chain dropped by closing isn't listed has these owners alone,
`is_js_iterator` and `collected`'s refusal of a helper's (ADR 0510). The
mutation `collected-closing-listed` survived: `consumer_drops_what_is_left`'s
`take(..).filter(..).map(..)` lost its `Iterator.from` here. A compiler that
panicked where the refusal changed the answer compiled the whole corpus,
its iterator forms and the lowering suites without panicking.

## Amendment: what's moved into a call is std's struct, held already (#606)

The rule above that held a call's value moved into another call whose
value the body drops, `Holder<Take<..>>`'s or `Peekable<Take<..>>`'s,
holds nothing any more. Since 1f3e1a7b0 an iterator type is one
representation wherever its values meet, so a `take` moved into a call,
generic or not, `wrap(it)` of a `Holder<I>` or of a `Holder` of a named
`Take<vec::IntoIter<_>>`, a `peekable()`'s or a `chain()`'s source given to
generic code, is std's struct, compiled, which steps what it holds as Rust
does and has no `hold()` (`held_parameters` already says so of a
parameter's). `$held(iterator_take(..))` was a call that did nothing.
The Nightly of 3c71ee8f9 found its mutation, `moved-into-owner-unheld`,
surviving: with it, `iterator_moved_into_a_holder` printed the same, and
of the whole corpus only that file's JS changed, losing its `$held`.

So `owned_where_run` asks no call any more: a local is owned where it runs
if the body drops it, or steps it by reference (#392). The two mutations
of the rule, `moved-into-owner-unheld` and `owner-closing-held-again`, are
gone with it. `iterator_moved_into_a_holder` still runs both holders
against native Rust, `kept` and `twice`, without the `$held`.

## Amendment: what closes a source steps it by its own steps (2026-11-10, #1239)

Generic code's `collect()` into a `B` the crate implements gives its
`from_iter` `$closing(Iterator.from(it), drop)`; the `from_iter` holds it,
`$held`, and drops it after its loop, as Rust drops its local. But JS's
protocol, `Iterator.from`, had already made `it`, a `$Steps`, close itself
when stepped to its end, whatever held it: so the zip a generic `collect()`
was given dropped what it held before the `from_iter` printed what it
summed, where native Rust drops it after. A `$Steps` JS's protocol steps
keeps its steps from before, `ownNext`, and `$closing` steps what it wraps
by them: what closes it is the `$closing`, held where its owner holds it.
One a variable holds and moves into a consumer stepped to its end, `let
mut c = ..; c.collect()`, still closes there, as Rust drops it. Held by
`from_iter_holds_what_generic_code_gives.rs` and the mutations
`closing-steps-by-closing-next` and `own-next-unkept`.

## Amendment: an iterator moved into what holds it is held (2026-11-11, #1270)

A temporary iterator is held, `$held(..)`, where the body drops it as it
runs on or steps it by reference (`owned_where_run`): not one only moved
into a call. But one moved into a value that holds it, `RefCell::new(it)`,
`Some(it)`, a struct's field, an array or a tuple, is dropped with that
value, where its owner drops it; unheld, what closes it, a `$closing`
generic code gave it, closed it as it was stepped to its end, so a
`from_iter` that kept it in an `Rc<RefCell<..>>` saw it dropped before it
printed what it summed. `moved_into_a_holder` is the one answer of every
place a temporary iterator can be moved into: an aggregate, or a call whose
value's type holds its type and isn't an iterator itself as the body's
bounds say (`may_hold_by_bounds`), so std's adapter of it, generic code's
`Take<Cycle<I>>` too, held through its own `hold()`, still closes what it
holds where it's done; not std's struct, compiled, which has no `hold()`
(#243), nor what std's own bodies move into theirs, a flatten's
`frontiter`, which they drop where they replace it. A `Box<I>` is `I` in JS, held as it is. Held by
`iterator_held_in_a_place_dropped_with_it.rs`, both builds, the six place
rewrites of `from_iter_holds_what_generic_code_gives.rs`, and the mutations
`moved-into-a-holder-unheld`, `boxed-iterator-unheld`,
`compiled-struct-in-a-holder-held`, `std-bodies-holder-held` and
`adapter-of-it-a-holder`. The crate's
own iterator so given, not an adapter of it, throws on its drop, #1286.

What std's own bodies move is std's, `is_std_crate`, not every body of
another crate: a library's generic body, which its consumer compiles from
the library's optimized MIR (ADR 0914), holds what it moves as the crate's
own does. Taking every non-local body for std's left a library's
`from_iter` holding none of the six but the box, `drop cell` before `cell
summed 3`, as the metamorphic net's `library` rewrite of
`iterator_held_in_a_place_dropped_with_it.rs` found in both builds (#1366).
Held by test/crates/pairs/library_iterator_held_in_a_place (the case's
items as a library and its consumer, against native Rust) and the mutation
`library-bodies-holder-unheld`.

## Amendment: a fold steps by its own steps too (2026-11-11, #1330)

Case ([0262](0262-when-rust-and-js-disagree.md)): results changed, so
Rust's are kept.

#1239 made what closes a source step it by its own steps, `ownNext`, the
ones before JS's protocol, `Iterator.from(it)`'s, made it close itself at
its end. `$foldBy`, std's default `fold(self, init, f)` of a `$Steps`,
still stepped by `next`. Given `Iterator.from(it)`, generic code's, the
step that found it done closed it inside the step. So a panic in what's
left of it, a `take(3)` of five items whose fourth's drop panics, ran as
a step's panic and dropped the accumulator: `drop total 6`. Std's fold
has its result by then and drops `self` after, and a panic there leaks
the result, never drops it. A `take` is `$taking` where its hint is
asked, and #1330's second defect makes that every `take` of its type
crate-wide. So the points table showed it beside a "hint given" case,
`vec | take | sum`.

`$foldBy` steps by `ownNext` where the value has one, and closes it after
its loop, where a panic leaves the accumulator as Rust leaves its result.
`test/corpus/fold_whose_rest_panics_as_dropped_leaks_its_result.rs` holds
the sum against native Rust in both builds; mutation
`fold-by-steps-closing-at-its-end`. #1330's points rows are off the list. (That mutation went on
2026-11-13: since #830, `$taking`'s own fold takes that sum, and nothing
reaches `$foldBy`'s `ownNext` with a close a program sees; ADR 1004's last
amendment.)
Whether a `take` is `$taking` by its type crate-wide or by the function
that makes it, #1330's other defect, which changes no answer now, is
#1333's.

## Amendment: a `Vec` generic code collects closes what it's given at its end (2026-11-11, #1239)

Generic code's `collect()` or `Vec::from_iter` into a `Vec` of what its
caller gives is one runtime call, `$collectVec(it, IItemSize,
IVecCollect, drop)` ([0145](0145-type-facts.md), amended), which steps what it's
given by JS's protocol, by its fold or its `next` as std's `SpecFromIter`
does, and so closes it at its end, where std's collect drops it, before
the `Vec` is given back; a `B: FromIterator`'s entry runs it inside
`$collecting`, which closed it where it was done (since removed: the collect closes it itself, ADR 0445, amended 2026-11-13). Of items that
drop, a panic closes it as each of std's ways unwinds, by `$foldToArray`
or `$collectDropping`. Held by
`vec_collect_as_its_caller_knows.rs` and the `generic-instances` row of
`fold_drops_moved_function_first.rs`, off the metamorphic net's list.

## Amendment (2026-11-12, #1507): a flatten's source dropped at its end

Case ([0262](0262-when-rust-and-js-disagree.md)): results change; Rust's
behavior preserved. Std's `Flatten` and `FlatMap` hold their source in a
`Fuse`, which drops it (`self.iter = None`) at its first `None`, and at the
end of a `try_fold` that runs it out, whether or not the flatten is held.
`$Steps`' `flatMap` set `ended` there and kept its source, which a held
flatten's source isn't closed by being done, so the crate's iterator was
dropped only with the flatten: `drop Flaky 2` after `end h` where native
Rust prints it before the `None`. Its `next` and `try_fold` now close their
source there; a `$closing` drops once however it's closed, so the
flatten's own close after drops nothing more, and a source a `by_ref()`
lends, whose close drops nothing of its owner, keeps it. Corpus case
`flatten_drops_its_source_at_its_end` against native Rust in both builds;
mutations `flatten-keeps-ended-source` and `flatten-try-fold-keeps-source`.

## Amendment (2026-11-13, #1647): a fallible collect of the crate's iterator drops it once

Case ([0262](0262-when-rust-and-js-disagree.md)): results change, so
Rust's behavior is preserved.

A `collect()` into an `Option` or a `Result` of a collection, std's
`GenericShunt`, closed what it steps, so what was left of it dropped where
a `None` or an `Err` stopped it. The crate's own iterator with a `Drop`,
of items that drop nothing, was closed there, which ran its drop. It was
dropped after the call too, as its consumer doesn't close it
(`consumer_closes`, which what's dropped after asks), so `drop Source`
printed twice where native Rust's prints it once.

Whether the crate's own iterator is closed is `consumer_closes`' answer
alone. Where its consumer closes it, the iterator is closed before the
collect (`mir_iter_method`); else it's dropped after the call, once. The
fallible collect closes only what isn't the crate's own.

`test/corpus/fallible_collect_drops_its_iterator_once.rs` holds it
against native Rust in both builds, of items that drop nothing and of
items that drop, stopped by a `None` and not. Mutation
`fallible-collect-closes-own-iterator-twice` (`lower/iterators/mir_steps/collect.ts`).

## Amendment: held past the end

A consumer that drops something it holds after its source's end, before
Rust drops the iterator, dropped it after the source (#2027).
`repeat(L(7)).take(3).is_sorted_by_key(|l| K(l.0))` printed `drop 7` before
`dropk 107`. std's `is_sorted_by` holds `last` in the closure its `all`
takes by `&mut self`, drops it as `all` returns, and only then drops
`self`, and with it the `Take` and its source. The JS closed the source
twice before that:

- **At the `take`'s limit:** `$taking` closes what it steps at its limit
  unless it's held.
- **At the chain's end:** `Iterator.from(items)` calls a `$Steps`'
  `[Symbol.iterator]()`, which arms it to close itself at its end.

**The rule.** Such a consumer steps its source by `$steppedHeld(items)`:
- the source is taken as it is (`$stepsOf`), never `Iterator.from`;
- it's held, so nothing in it closes at its end;
- the consumer closes it, `return()`, once what it holds is dropped.

A `$lent` source's hold stops at the `$lent`, so its lender's iterator goes
on as it was.

**Who uses it.** `$isSortedBy` uses it on every path. That covers the keys
`is_sorted_by_key` drops (#2027), items with a destructor of `is_sorted` and
`is_sorted_by` (#948), and items without one, where only the closure's
captures drop past the end (#2029). Before #2029, that last path closed the
iterator where two items were out of order, before `all` dropped the
closure: a wrong drop order.

**Who will.** The other consumers that drop something past the end are
refused today with items that have a destructor:
- the comparison family, `cmp`, `partial_cmp` and `lt`, of an iterator that
  ends first while the other gives one more;
- `eq` of two iterators whose lengths std asks first.

Each takes this helper when it's lifted.

**Agreeing with native.** A probe found `zip` of a shorter `take` agrees.
#2027's own report found `max`, `last`, `all`, `any`, `for_each` and `fold`
agree too: they give back what they hold, or hold their closure's drop
apart.

Held by `test/corpus/sorted_keys_dropped_before_their_source.rs`:
- a `repeat`'s value;
- a `Vec`'s rest past the limit;
- a `take` moved in from a variable;
- an out-of-order run.

Mutations `stepped-held-unheld` and `is-sorted-by-steps-armed`.

## Amendment: what std's compiled struct holds (2026-11-18, #2039)

Generic code's `it.fuse()` of a `FusedIterator` dropped its source when
the source was stepped to its end, where Rust's `Fuse` keeps it until the
`Fuse` drops. A `map` whose closure captured a `D` printed `drop 1` before
`drained`, where native prints it after. Std's struct holds what it's
given past its end, and drops it by the drop it's given, where Rust's drop
glue or std's own code drops it. Two things closed it first:

- **`Iterator.from(it)`.** Generic code gave std's compiled body its type
  parameter's value so (#1962). That calls a `$Steps`' `[Symbol.iterator]()`,
  which arms it to close itself at its end, as JS's own by-value consumers
  take it.
- **Its own `done`.** An adapter made in place, a `$closing` its caller
  made, closes itself at its end unless it's held. A held `Fuse` holds its
  source, as a `$Steps`' `hold()` holds its `iter`; but one in an array,
  `[it.fuse()]`, isn't held, and its source closed as it was done.

**The rules.**
- A type parameter's value given to std's compiled body as its `self` is
  `$stepsOf(it)`: an array's `$iter`, a `$Steps` as it is, anything else
  JS's iterator of it, `Iterator.from`'s, which takes one with a `next`
  alone (ADR 0061); never a `$Steps` armed.
- A value moved into std's compiled struct, as the call makes it, is a
  holder's: held where it's made, or where its function starts
  (`moved_into_a_holder`), as a value moved into an aggregate is. It's
  followed through each temporary MIR moves it into whole, `fuse(move _4)`
  of `_4 = move _2`. Not into an adapter the body makes in place, a JS
  iterator whose `hold()` holds it.

**One output form per construct.** A type parameter's value is stepped as:
- `$stepsOf(it)`, given to std's compiled struct, which holds it and drops
  it where Rust does, so it mustn't close itself at its end;
- `Iterator.from(it)`, in generic code's own chain of JS's iterator
  helpers, `Iterator.from(it).map(f)`, whose helpers never call
  `[Symbol.iterator]` and so need it armed to close at its end, where
  Rust's chain consumed in place drops its source.

JS's own by-value consumers, `Array.from`, `for..of` and a spread, arm a
`$Steps` themselves when they take it. That's how a `Fuse` given back to
its caller and collected there, or left by a loop's `break`, drops its
source with it, as native does.

89 corpus snapshots move, each one of the two forms: `Iterator.from(it)` to
`$stepsOf(it)`, and a `$held(..)` of a value given to std's struct. Each
runs as native, in both builds; none's output moves. React's port doesn't
move.

Corpus case `fuse_holds_its_source_in_generic_code` holds, against native:
- a `fuse()` of a `FusedIterator` source, std's `Map` and a `map` its
  caller makes in place, and one in an array;
- `map`, `take` and `zip` in generic code, the zip made in place;
- non-fused sources, the crate's own and a boxed `dyn`, dropped at their
  first `None` as std's does;
- a `Fuse` dropped before its source's end;
- held locals in code that isn't generic;
- a `Fuse` given back, collected by its caller, and left by a `break`;
- a panic at its end.

Mutations `compiled-self-armed`, `compiled-struct-not-a-holder`,
`moved-through-temporaries-unseen` and `made-in-place-a-holder`. Generic
code's `fuse()` where no caller gives `IFused`, an impl's own method's,
still takes `false`, std's non-fused `Fuse`: #2042, as itertools'
`ZipLongest::rfold` reaches it.

## Amendment: std's default fold holds what it folds (2026-11-21, #2232)

Case ([0262](0262-when-rust-and-js-disagree.md)): results changed, so
Rust's are kept.

#1330 had `$foldBy`, std's default `fold(self, init, f)`, step by
`ownNext` and close what it folds after its loop. Something it steps
could still close at its own end inside a step. A generic `Box<I>` is
lent as `$lent($closing(src, drop), "boxed")`, and a box doesn't pass
`fold` on, so the box's fold is `$foldBy` over the lend. That fold steps
the `$closing`, which closed `I` when it found it done. When `I`'s drop
panicked there, it ran as a step's panic, and the fold dropped the
accumulator: `drop acc 15`. Std's `fold(self)` has its result by then.
It drops the box, and so `I`, as it returns, and a panic there leaks the
result. The in-place form behind this, `Iterator.from($lent($closing(..),
"boxed").map(..))`, is chosen by a crate-wide fact: the same adapter type
stepped from its back anywhere in the crate. So the points table showed
it only beside a `next_back` case. Generic code's `Box<I>` became a
`DoubleEndedIterator` with #2227.

A crate-wide per-type fact may choose an in-place form only if that
form is right for every source of the type. So the form is what's fixed,
not the fact:

- **`$foldBy` holds what it folds before its first step,** as `$consume`
  and `$steppedHeld` do. Nothing it steps closes at its end inside a
  step. It's closed once, after the loop, after what drops with `f`,
  which is std's order. An adapter whose `next` drops its source, std's
  `Fuse` (a flatten's source) or a chain's first half, drops it there
  itself, held or not.
- **A `Box`'s lend, held, holds what it owns,** since the box's drop is
  its value's. The lend's `hold` holds it, not its `from`, which says
  what std's `SourceIter` reaches, and no `Box` is one. A `&mut` lend's
  hold still stops at the lend.

Neither change moves the JS the compiler writes. On the points table,
five generic boxed rows now run as native: `inspect | fold`, `chain | fold`,
`step_by | fold`, `flat_map array | fold` and `filter_map | fold holding`.
`test/corpus/boxed_source_dropped_after_its_fold.rs` holds `fold`,
`last` and `sum` against native Rust in both builds, beside a
`next_back` of the same adapter type. Mutations `fold-by-unheld` and
`boxed-lend-hold-stops`.

## Amendment: a fallible collect's closing is the one answer of who drops (2026-11-25, #2561)

A `collect()` into an `Option` or a `Result` closed what it steps unless
it was the crate's own iterator (#1647). `dropped_after`, which says
whether the iterator is dropped after the call, asked `consumer_closes`
instead, and that answered no for a type parameter's iterator whose items
drop nothing. So generic code's `i.collect::<Option<Vec<u32>>>()` of the
crate's iterator with a `Drop` closed it and dropped it again after the
call: `drop at 3` twice where native prints it once, a `Result` collect
too.

`fallible_collect_closes(output, iter_ty)` (closing.rs, beside `consumer_closes`) is now the one
answer. It's true where closing drops something, which is what
`closes_to_drop` says: a type parameter's iterator, or one that owns
items with a destructor. The crate's own iterator is excluded, as before,
and so is a compiled std adapter's struct, which closing leaves as it is.
The collect closes where it says so, and `dropped_after` drops nothing
after. Of items that drop, a type parameter's is closed by its consumer
and by the collect. `closing` gives a value that's closed already back as
it is, and `closing-closes-again`, a nightly survivor since 2026-11-22, is
caught by this case.

`test/corpus/generic_fallible_collect_drops_its_iterator_once.rs` holds
an `Option` collect, one of a `take(2)`, a `Result` collect, and items
that drop, of the crate's iterators and a `Vec`'s, against native. The
whole corpus's JS is unchanged otherwise. Its `loud-drop` rewrite finds
#2564, which is on main without this change: a panic in the iterator's
`Drop` as the collect closes it leaves what was collected undropped. It's
listed known in test/known/metamorphic/2564.txt. Mutation
`fallible-collect-closed-and-dropped-after`.
