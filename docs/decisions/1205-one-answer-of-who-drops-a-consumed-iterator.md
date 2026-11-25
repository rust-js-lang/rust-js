# 1205. One answer of who drops a consumed iterator

Date: 2026-11-25.

Status: accepted (#2572, its four steps). Amends
[0465](0465-dropping-an-adapter-is-closing-it.md) and
[0570](0570-a-closing-source-is-dropped-when-done.md).

## Context

When std's code takes an iterator by value, its callee drops it. rust-js
lowers the callee to a runtime helper and chooses who stands in for that
drop. The helper can close it, `$closing(it, drop)`, at its end and as a
panic unwinds. The caller can drop it after the call (`dropped_after`).
Or nobody does, where it's lent or holds nothing to drop. Exactly one
must.

Each side asked its own predicates. The closing side asked
`consumer_closes` from `dropping_source` and a std combinator's lowering,
`keyed_consumer_closes` of a keyed extreme, and `fallible_collect_closes`
of a fallible `collect()`. `dropped_after` asked them again, negated,
together with `try_processed_into`, `extend`, the crate's own `fold` and
the params-only, held-function, `closed_on_drop` and compiled-struct
rules. Each side combined them its own way. #2561 was a `collect()` that
closed where `dropped_after` didn't know it would, and #2580 a fallible
`sum` or `product` that nothing asked as a fallible `collect()` is asked.

## Decision

One owner, `iterators/mir_steps/consumed.rs`, answers who drops what a
call of std's code takes, `consumed_drop(Consumed) -> ConsumedDrop`:

- `Nobody`: lent, or nothing to drop;
- `Closes`: its consumer closes it, `$closing`;
- `Shunted`: a fallible `collect()` into a collection whose own
  `from_iter` runs code gives it std's `GenericShunt`, whose drop it is
  (#1588);
- `OwnFold`: the crate's own `fold` or `rfold` takes it and drops it
  (#162);
- `After`: the caller drops it after the call.

A `Consumed` is the call's facts: its callee, its iterator method, the
iterator's type, the call's arguments', what it gives, whether it's lent,
and the JS of what it's given where that's asked. Every reader passes
them whole, so none answers from part of them. A collect lowered apart from
its call, a `from_iter` entry or a fallible collect's items, is std's
`Iterator::collect` (`Consumed::collecting`).

Both sides read the one answer. `consumer_closing` closes where it's
`Closes`, and so do `dropping_source`, a std combinator's lowering, an
`rfold`'s receiver, a keyed extreme and a fallible collect.
`dropped_after` is `After`. ADR 0190's requirement that callers give a
generic iterator nothing that drops stays with `dropped_after`, where its
answer is `Nobody` of an iterator that holds only type parameters' values:
a collect lowered apart from its call asks of a stand-in, `I`, which no
caller gives.

Step 1 computes the answer from today's predicates, in `dropped_after`'s
order. One snapshot moves, `fallible_collect_into_own_from_iter`, which
was both shunted and closed, two claims on one iterator. It is now
`Shunted` alone, as the design's rule 4 says. A `from_iter` of the crate's
own that panics part way drops what it took and what's left in native's
order (`fallible_collect_into_own_from_iter_that_panics`). The case of
ADR 0262 is "results change": the answer is Rust's.

## Amendment: step 2, a fallible `sum` and `product` (2026-11-25, #2580)

A fallible `sum` or `product` into an `Option` or a `Result` is std's
`try_process`, as a fallible `collect()` is, and nothing asked it so.
Generic code's dropped its source after the call, outside `try_process`'s
frame. Where the source's own `Drop` panicked, the kept `Err` was never
dropped, which native drops as that panic unwinds. Rule 4 is now of all
three (`Collect | Sum | Product`), so the consumer closes it.

`dropping_source` returned early of items without a destructor, before
asking the answer, so a `product` of `Option<u32>`s, `Closes` now, was
closed by nobody, and its source wasn't dropped as its step panicked. The
early return now skips only `check_leaves_none`, and the answer is asked
of every consumed iterator. The consumer net's twelve #2580 cells run as
native's, and its whole `consumers` table passes. Held by the corpus cases
`generic_fallible_sum_drops_the_kept_err_as_its_source_drop_panics` and
`generic_fallible_product_drops_its_source_as_its_step_panics`, and the
mutations `consumed-fallible-sum-unclosed` and
`dropping-source-answer-skipped`.

## Amendment: step 3, its rules its cases (2026-11-25)

`consumer_closes`, `keyed_consumer_closes` and `fallible_collect_closes`
are inlined into `consumed_drop` as its cases, and gone, so no rule of
who drops a consumed iterator is written outside it. Their mutations moved
with their lines (`consumed.ts`), each caught where it was. The test
"who drops a consumed iterator is consumed_drop's answer alone"
(test/architecture-iterators.test.ts) holds it. No function that says
whether a consumer closes, `fn ..._closes`, is written in the compiler.
MIR's `dropped_after` reads the answer. Its readers are `consumer_closing`
(every closing site's), an `rfold`'s and `dropped_after`. No output moves.

## Amendment: step 4, two forms of a source's close (2026-11-25)

The runtime closed a source 88 ways, each `.return()` or `.return?.()`
written where it closed. Now each is one of two helpers in steps.js:
`$closeHolding(it, ...held)`, a collector's, which drops what it holds
where the close panics, as std's callee's frame does; or `$closeBare(it)`,
of what holds nothing of its own beside it. `$collectOptions` and
`$collectResults`, which held what they collected by hand, take the
first. The other 86 take the second, mechanically, so no behavior moves.
A runtime test holds that no `.return(` is written outside the two
helpers' bodies.

`$closeBare` claims that what closes holds nothing, so each collector it
closes was checked. The consumer net's `source drop` cells, run alone
against std's bodies, import 14 of them, and 7 more through those
helpers' calls (`$extremeBy`, `$consume`, `$foldBy`, `$collectUnwinding`,
`$eachOf`, `$inPlaceSource`, `$consumed`). Of the rest:

- `$stepsAround` and `$tryProcessing` close as a `$Steps`'s callback, an
  adapter's, which holds nothing.
- `$sortedSteps` drops its peeked entry beside the close by `$inTurn`,
  whether or not the close panics.
- `$roomForCounted` closes before anything is written.
- `$decodeUtf16` of a source with a destructor is refused at compile time.
- `$findDropping`'s found item is dropped as native drops it, which a
  corpus case shows against native
  (`find_of_a_source_whose_drop_panics_drops_what_it_found`).

None changed behavior, so no P0 was found.

## Steps

1. `consumed_drop` from today's predicates, both sides reading it (this
   amendment).
2. Rule 4 over a fallible `sum` and `product` into an `Option` or a
   `Result`, which fixes #2580 and takes its 12 known cells off the net's
   list.
3. The predicates inlined into its cases and deleted, with an
   architecture test that only `consumed_drop` decides.
4. `$closeHolding(it, ...held)` across the runtime's closers, with a
   native-compared case for any whose behavior changes.

## Consequences

Who drops a consumed iterator is one answer. A rule added to it is seen
by both sides, so neither can drop what the other does. Its mutations are
`scripts/mutations/lower/iterators/mir_steps/consumed.ts`: the rules
`dropped_after` held moved with it, and `consumed-shunt-closed` is new.
