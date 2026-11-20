# 1176. An in-place form answers the std type it stands for

Status: Accepted (2026-11-20, the lead's decision for #2112's Change A,
step 3, phase 1). Amends [0515](0515-map-while-and-kept-somes.md) and
[1092](1092-the-iterator-protocol-is-one-contract.md).

Case ([0262](0262-when-rust-and-js-disagree.md)): R. Which of the crate's
steps std calls is observable, so a form must call exactly those.

## Context

The recognizers make several std adapters in place as one runtime method:
`$Steps.filter` stands for `Filter`, and also for the `Some`s of a
`filter_map`, a flatten of `Option`s and a `flat_map` to them,
`src.map(f).filter((item) => item != null)` (ADR 0515). The runtime net
(test/iterator-steps.test.ts) held each runtime form to the std adapter
it's named for alone, so `$Steps.filter`'s own `count`, std's
`Filter::count`, which asks its source's `size_hint` first, was right for
`Filter` and wrong for the other three, whose `count` is their fold.

Consumers that know the type chose around it (#2197's fix: the count
where it's made asks `counts_by_its_own`). Generic code can't: given the
form, it counts by `$count`, the form's own (#2199). So a crate-wide fact
that makes a crate iterator knowing its hint, and its adapters in place,
asked the crate's own `size_hint` where std never does, the fourth such
wrong answer (#2157, #2193, #2194, #2197).

## Decision

**A form made in place answers the catalog row of the std type it stands
for, for every source, so no consumer has to choose around it.** The
first such answer is the `Some`s:

- **`$somes(it)`**, a runtime form: a filter of what's there,
  `$filterEnds`' where `it` knows its back, whose own `count` is its fold,
  std's `FilterMap`'s and `Flatten`'s; of a JS iterator that knows no
  back, JS's own `filter`, which has no `count`.
- **It's made where its source's `size_hint` may run the crate's code, by
  its type** (`hint_reaches_own_where`, of a type parameter's and what a
  `dyn` holds too). That's true of every value of the type alike, whatever
  form a crate-wide fact made it, so no fact chooses the form. Elsewhere
  the `Some`s stay `filter((item) => item != null)`, as JS writes them:
  asking a hint that runs nothing changes nothing.
- **Generic code's sources** (a type parameter's, a dictionary's) are
  `$somes`, as the site can't know what a caller gives: the readable
  filter could give a wrong answer there, and correctness decides (the
  lead, 2026-11-20).
- **The net holds it**: `$somes` of a map, and of a map that knows its
  back, run against `FilterMap`'s row of std's catalog.

Phases 2 and 3 (#2112): one owner names each in-place form's std type
(`in_place_forms`), the net enumerates those pairs from it, and consumers
call the form's step instead of re-deciding from types.

## Why

- **Correct by construction.** A form that answers its own type's row is
  right whichever consumer reaches it, generic code included, which no
  consumer-side rule can be.
- **Readable where it's free.** The type's answer leaves every slice,
  `Vec`, range and std source's `filter_map` as JS writes it. Only crate
  iterators with their own `size_hint` and generic code's sources move:
  in the corpus, two cases (generic functions); in React's port, none.

## Alternatives

- **The form's answer** (does the made Expr carry the crate's
  `size_hint`?), with the type's. Tried: over the whole corpus it never
  changed a line the type's answer didn't, and it reads JS shapes.
- **Always `$somes`.** Correct, but every `filter_map` of a slice would
  lose upstream's `filter((item) => item != null)`.
- **Not in place there** (std's compiled struct). Correct, but the
  struct's source form would have to follow the same choice.

## Consequences

- Corpus `flatten_of_options_counted_asks_no_hint` holds it (counts where
  they're made, and in generic code); `filter_ends` and
  `filter_map_of_given_from_its_back` moved to `$somes`; React's port is
  unchanged.
- knowing.rs makes `$somes` of a source stepped from its back of that
  source made knowing it, as it made `$filterEnds`.
- Mutations: `somes-counted-as-a-filter`, `somes-of-a-back-forward-only`,
  `somes-never`, `somes-of-any-type`, `somes-not-made-knowing`.

## Amended: one owner names each form, and the net runs it both ways (2026-11-20, step 3's 1b and (d))

**The owner.** `src/lower/iterators/in_place_forms.rs` names each form
lowering makes a std adapter or consumer as in place (`InPlaceForm`),
the std types it stands for (as std_steps.txt names its rows), and
where it's chosen (`Where`).
- `in_place_form` is the one decision between forms: the `Some`s,
  `$somes` or JS's filter; a `step_by`'s, `$stepBy` or a filter by
  index (`steps_by_index`, whose conditions moved here from
  `combinators/adapters.rs`); and `nth`'s, `$nth` or `drop(n).take(1)`.
- `flattened_form` chooses between `$flatMap` and JS's `flatMap`.
- adapters.rs emits what they answer, byte for byte as before.

**The table.** `src/lower/iterators/in_place_forms.txt` is the owner's
table (`BLESS=1 cargo test in_place_forms`, the contract-table
pattern). test/iterator-steps.test.ts reads it:
- The net's forms are the table's pairs, both ways.
- Each form runs as it's emitted against its std type's row, over the
  sources its `Where` allows. Where the hint runs nothing, the source's
  `size_hint` calls are left out of both traces. Where the inner
  iterators are plain, only sources of no own steps are used. A filter
  by index of an array's items gives std's items.
- Consumers run over a map of the source, held to `Map`'s row of the
  method.

**The guard.** Each `Where` is a fact of every value of the source's
type, or MIR's, never a crate-wide fact (the lead). A test holds each
`Where`'s APIs to owners of class 1 or 2 in docs/architecture.md, so the
net's exclusions can't hide a form a crate-wide fact chose, #2197's
kind. A choice may also ask what narrows it further. A `step_by`'s
filter asks `made_knowing_its_hint`, which only narrows, and its
`Where` is `bounded_by_items`.

**New native rows.** std's catalog gains `Flatten<Option>` and
`FlatMap<Option>` (test/std-steps.rs): std's `FlattenCompat` over
`Option`s steps its source by `next`, where `FilterMap` steps it by
`find_map`, a `try_fold`.

**What it found.** #2205 (P0): the `Some`s' forms answer `FilterMap`'s
row but not `Flatten<Option>`'s or `FlatMap<Option>`'s. Through a
`skip` or a `step_by` of the crate's iterator with its own `nth`, made
in place where #2197's type-keyed hint fact puts the chain, the crate's
`nth` is asked otherwise. Its rows are in the net's `differs` under
#2205 until it's fixed. Every other new pair answers its row: the
`Some`s as JS writes them for `FilterMap`, the filter by index,
`$flattenEnds` and JS's `flatMap` for `FlatMap`, `drop(n).take(1)`
and `find_map`'s map and find.

Mutations: the catalog's `step-by-*`, `nth-own-by-drop`, `somes-*` and
`flat-map-*` moved with their code to
scripts/mutations/lower/iterators/in_place_forms.ts. New ones:
`in-place-somes-filter-of-any-source` and
`in-place-somes-not-a-flatten`, caught by the table's unit test.

## Amended: a flatten of `Option`s is std's `Flatten` of them, and the net holds what a step answers (2026-11-21, #2205)

**The form.** A flatten of `Option`s, or a `flat_map` to them, is std's
`FlattenCompat` over one-shot inner iterators (core's flatten.rs, the
`OneShot` specialization). It steps its source by `next` and
`next_back`, where a filter of the `Some`s steps it by `try_fold`. A
`skip(n)` between asks the crate's `nth(n)` by `next`, but `nth(n - 1)`
and a `next` by `try_fold`; a `step_by`'s first `next` asks `nth(0)`.
- **`$someItems(it)`**, a runtime form: `$somes`' filter of the `Some`s,
  whose folds are its source's with the function moved into them, as
  std's `flatten_one(fold)` is, and whose count and last are their fold.
  But its `next` and `next_back` step its source by `next` and
  `next_back` until one gives a `Some`, fused as std's `Fuse` holds it.
  Its `advance_back_by` is its source's `try_rfold`, as its `advance_by` is
  its `try_fold`.
- **Its hint is std's one-shot one**, none to its source's upper bound,
  none at all once it's ended. The generic `FlattenCompat`'s, `$flatHint`'s,
  is `(lo, None)`.
- **Not `$flatMap` of one-item arrays.** That was tried first. Its steps
  answer the rows, but its hint is the generic one, and its fold drops
  what drops with the consumer's function after the source's rest, where
  std's one-shot fold moves the function into the source's fold, so it
  drops first. The conformance table's `boxed dyn | flatten option | fold
  holding` rows found that.
- **It's chosen** where a crate's iterator within the source writes `nth`
  or `nth_back`, or a type within is unseen (`own_nth_within`, own_steps.rs,
  class 1), or where the source's hint may run the crate's code
  (`hint_reaches_own_where`). Elsewhere it's JS's filter, which asks
  what std's `Flatten` asks there.

**The owner's table** gives each (std type, form) pair its own `Where`
(`stands_for`), as one form stands for one type wherever it's made and
for another only where their steps agree:
- `$somes` and `$somes` of ends stand for `FilterMap` alone.
- JS's filter of the `Some`s stands for `FilterMap` where the hint runs
  nothing, and for `Flatten<Option>` and `FlatMap<Option>` under a new
  `Where`, `StepsRunNothing` (`own_nth_within` and
  `hint_reaches_own_where` false). The net runs that over sources that
  write only `next`, `next_back` and `size_hint`, the hint left out.
- `$someItems` and `$someItems` of ends stand for the two always.
- `chosen` is gone.

**The net holds what a step answers.** std's catalog (test/std-steps.rs)
now prints what `size_hint` and `len` answered, `= (0, Some(4))`, after
what they asked. It gains a step, `size_hint_after_next`, the hint once a
`next` has begun it. The net compares each form's answer with std's,
`usize::MAX` by name. A trace alone missed the hint `$flatMap` would have
given here. Every other form's answers agree with std's, fresh and after
a `next`.

**A known difference no consumer reaches.** JS's filter of the `Some`s
doesn't fuse its source, where std's `Flatten` does. After its source's
`None`, a `next` asks its source again, and the source is dropped when
the filter is, not at that `None`. `$someItems` steps it no more after it,
but drops it with itself too, and a fold after its end asks its source
again. No in-place chain reaches this today: a chain bound to a variable
is std's compiled structs, and every in-place consumer stops at the first
`None`. A future consumer that steps an in-place flatten past its end must
check this first.

**What `own_nth_within` decides alone.** A boxed iterator of the crate's
with its own `nth` and no `size_hint`, `Box::new(Counter(..)).skip(1)`, is
made in place with no hint fact, and its `nth` alone chooses `$someItems`.
Elsewhere, an in-place chain over a crate iterator with its own `nth` is in
place because a type-keyed hint fact put it there, which needs the crate's
`size_hint`, so `hint_reaches_own_where` answers too. `$someItems`' hint is held by
the net, not by a corpus case: every hint asked of an in-place flatten of
`Option`s is asked of std's compiled structs. So is its `next_back`: a
filter's, its source's `rfind`, asks what `next_back` does of every source
a program gives it, a `skip`'s length once either way. Since #2206 the net
runs against a mutant's runtime, and `some-items-hint-unended` and
`some-items-back-by-its-filter` are caught by it alone.

**Moves.** Corpus `flatten_of_options_counted_asks_no_hint`'s flattens
(both in place over the crate's iterator and given to generic code) are
`$someItems`; nothing else moved. React's port is unchanged.

Corpus: `flatten_of_options_steps_its_source_by_next`, the issue's case
from either end, boxed, and in generic code. Mutations: `flatten-of-options-a-filter`,
`flatten-of-options-items-of-any-type`, `some-items-filtered`,
`own-nth-within-never`,
`in-place-somes-filter-flatten-of-any-source` and
`in-place-some-items-not-a-flatten`. `in-place-somes-not-a-flatten`
is gone with the list it mutated. `in-place-somes-filter-of-any-source`,
`somes-never`, `somes-of-any-type`, `filter-map-ends-unmade` and
`somes-not-made-knowing` were retargeted.
