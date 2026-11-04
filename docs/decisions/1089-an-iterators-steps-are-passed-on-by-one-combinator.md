# 1089. An iterator's steps are passed on by one combinator, and std's steps are one catalog

Status: Accepted (the user's decision, 2026-11-04, through m6-react: the
OwnStep refactor). Amends [0669](0669-an-adapter-steps-its-source-as-stds-does.md)
and [0465](0465-dropping-an-adapter-is-closing-it.md): how an adapter of
the runtime passes a step on.

Case ([0262](0262-when-rust-and-js-disagree.md)): R where a step was lost,
results change, so Rust's behavior is kept; N elsewhere, the JS a program
compiles to is the same byte for byte.

## Context

An iterator rust-js makes is a `$Steps` (ADR 0669): `next`, and the steps
std's adapters specialize, `fold`, `try_fold`, `rfold`, `try_rfold`,
`nth`, `nth_back`, `last`, `count`, `advance_by` and `advance_back_by`,
each given what its Rust takes: an item's drop, of what it passes over, or
an accumulator's `unwind`, what a stage's panic strands (#423). Std's
semantics of each step are kept in three places, and each kept them by
hand:

- **Each runtime adapter passed on the steps it was written with**, about
  113 assignments, `rev.fold = (init, f, unwind) => it.rfold(init, f,
  unwind)`, each with the arguments its author thought of. `$mapEnds` had
  no `fold` (#621), `$enumerate` dropped `unwind` (#626), `$skipping` lost
  the items' drop (#492), and `$rev`'s `nth` its `drop`.
- **Step lists**: `$given`'s own steps, `$iterator`'s thirteen positional
  steps, knowing.rs's `takes`, and the dictionary of a type parameter's
  steps (#520, #522, #649).
- **Per-step recognition**: `folds_its_own`, `nths_its_own`,
  `lasts_its_own`, `counts_its_own` and `may_fold_its_own` each answered
  for a type parameter and for std's adapters in its own way (#496, #505,
  #521).

m6's classification of 2026-11-04 counted 21 P0s of this family (F-ITER);
the iterator matrix (ADR 1088) found more (#646, #649, #650, #653).

## Decision

**One runtime combinator, `$forward(adapter, source, steps, each)`
(`src/runtime/forward.js`), is the one place a step is given an adapter.**
Each adapter declares the steps std specializes for it, as one object:

- a string names `source`'s step it passes on, `fold: "rfold"` of a
  `Rev`'s; it's given only where `source` has that step, with **every**
  argument it's called with, so an item's drop and an accumulator's
  `unwind` always go on, and it's the crate's own (`ownNth`, `ownFold`, …)
  where `source`'s is;
- a function is the adapter's own specialization, std's `Take::nth`;
- `undefined` leaves the step to std's default, `$Steps`' (fold by `next`,
  `nth` by `advance_by`, `last` and `count` by `fold`), or from the back,
  `back_steps.js`' helpers.

Given `each`, what the adapter does to each item, a `map`'s or a
`filter`'s, every fold passed on by name is given `each(f, unwind)` for its
function, `unwind` found where that step takes it. Such a fold isn't the
crate's own: the adapter holds its function too, which the crate's `fold`
that takes the source doesn't drop.

`test/architecture-iterators.test.ts` ("an iterator's steps are given by $forward
alone") fails on any assignment of a step outside `forward.js`.

## Why

Each P0 of the family was a step, or an argument of one, an adapter forgot
to pass on. Passing every argument by rest parameters makes the second
impossible; one declaration per adapter, where a step missing is a line
missing from one object, makes the first visible, and the architecture
test keeps a new adapter from writing its own.

Alternatives: a class per adapter, whose methods override `$Steps`', was
rejected: the runtime's adapters close over their state (`first`,
`sides`, `n`) and `$stdIterator`'s struct is a plain object made a
`$Steps` by its prototype, so methods would need that state as fields,
which `$copy` and `$become` copy. A table of each adapter's forwarding
built at load time was rejected: most steps are specializations, not
renames, and a table would separate each from its comment.

## Consequences

- Every runtime adapter, `$lent`, `$rev`, `$taking`, `$skipping`,
  `$lazyChain`, `$mapEnds`, `$chainEnds`, `$stepBy`, `$filterEnds`,
  `$enumerate`, `$closing`, `$stdIterator`, `$iterator`, `$Steps`' `map`,
  `filter`, `flatMap`, `take` and `drop`, the lazy `take_while`,
  `map_while`, `skip_while` and `scan`, `$folded` and `$lazyRange`, gives
  its steps by `$forward`.
- A `rev()` passes on the `drop` its `nth` is given to its source's
  `nth_back`, and says its `fold` is the crate's own where its source's
  `rfold` is, so a closing over it drops nothing after it.
- The mutations of each adapter's forwarding name its line in its
  `$forward` declaration, each as it meant; `forward-arguments-dropped`,
  `forward-own-unsaid` and `forward-step-it-lacks`
  (`runtime/forward.ts`) guard the combinator.

## Amendment: what may panic is the type's answer, and what a step takes is closed as std's is (2026-11-04, #646, #650, #653, #671)

Case ([0262](0262-when-rust-and-js-disagree.md)): R. Each changed results, so Rust's behavior is kept.

- **What may panic as an iterator is stepped is `steps_may_panic`'s alone**
  (`mir/iter/collect.rs`), the iterator's revealed type's answer. A
  collect's unwinding (`unwinding_collect`) and a fold's `unwind`
  (`stranded_drop`) also read the JS: `chain_may_throw` said an arrow may
  throw, and `closes_a_source` said a source is `$closing(..)`. A `fuse()`,
  `peekable()`, `chain()` or `zip()` of a `Vec`'s `into_iter()`, mapped by a
  closure that panics and collected into a `String`, a `HashSet` or a
  `HashMap`, showed no `$closing`. Std's compiled struct holds the `$iter`
  and drops it as it's closed, so the collect was `Array.from(..)`, and what
  the source had left was never dropped (#653, 4,672 of the matrix's case-builds).
  Now a collect of an iterator whose steps may panic, and whose type holds
  what has a destructor, is `$collectDropping`. A `last()` of one is `$last`,
  std's default of its `fold`, which closes it as it unwinds.
  `chain_may_throw`, `closes_a_source` and `callbacks_in` are gone.
  `test/architecture-iterators.test.ts` ("whether an iterator's steps may panic is
  steps_may_panic's alone") keeps both gates from being given the JS.
- **`rfold(self)` closes what it takes**, as `fold` does. `back_step` asks
  `consumer_closes`, so a `Vec`'s rest and the crate's double-ended iterator
  with a `Drop` are dropped at its end and as its closure's panic unwinds. A
  generic `rev()` of a type parameter is closed by the parameter's drop, as
  `Rev` drops what it holds (#650).
- **A copy that's its source, stepped from its back, makes its source
  knowing its back**: `nums.iter().skip(1).copied().rfold(..)` was
  `$foldBack(nums.values().drop(1), ..)`, JS's `drop`, which has no back;
  now `$skipping(..)` or `$taking(..)`. A filter of a reversed `Vec`'s
  items with a `Drop`, `rev()`ed again and held, is made knowing its back
  as one of the items is (`back_stepped.rs`'s `dropping_items`, #646). A
  `cloned()` of a `skip()` stepped from its back is refused, as before.
- **The crate's own `fold` and `count` are called where std's adapters call
  them** (#671). A `step_by()` of a source whose `fold` is the crate's own
  is `$stepBy`, which steps it by `next` and `nth` as std's `StepBy` does,
  not a filter by index, whose fold called it. A `skip()` and an
  `enumerate()` say their `count` is their source's own (`ownCount`), so a
  `skip()` of either calls it. A closed iterator's own `count` is given the
  items' drop.

The readability baseline's `$iter` is 75.0 of 10,000 lines, from 74.9. Each
added one is a `Vec`'s items closed under a collect whose closure may panic,
`$collectDropping($closing($iter(v), ..).map(f))`, where the eager
`v.map(f)` dropped nothing the panic left.

Corpus cases: `rfold_and_rev_close_what_they_step.rs`,
`compiled_adapters_closed_as_a_collect_unwinds.rs`,
`copied_skip_take_and_filter_stepped_from_back.rs` and
`own_fold_and_count_where_adapters_call_them.rs`. Mutations:
`rfold-receiver-unclosed`, `generic-rev-unclosed` and `copy-of-skip-backless`
(`lower/mir/iter/backs.ts`), `filter-of-a-rev-backless`
(`lower/analysis/back_stepped.ts`), `collect-holding-unclosed`
(`lower/mir/iter/collect.ts`), `last-holding-unclosed`
(`lower/mir/iter/closing.ts`), `step-by-of-own-fold-filtered`
(`lower/combinators/adapters.ts`), `skip-own-count-unsaid`,
`enumerate-own-count-unsaid` and `closing-count-drop-unpassed` (runtime).
`hidden-stage-unseen` and `callbacks-uncounted` are gone with the JS count
they guarded; a hidden stage is the type's, `fold-stage-by-type-unseen`'s.

## Amendment: which own step an adapter reaches is one catalog's (2026-11-04)

Case ([0262](0262-when-rust-and-js-disagree.md)): N. The JS is the same
byte for byte.

`folds_its_own`, `rfolds_its_own`, `try_rfolds_its_own`, `nths_its_own`,
`nth_backs_its_own`, `lasts_its_own`, `counts_its_own` and
`may_fold_its_own` are gone. Each kept its own list of std's adapters and
its own answer for a type parameter and a `&mut`. They're now one query,
`reaches_its_own(ty, step, reach)` (`recognition/own_steps.rs`), over one
enum, `OwnStep`:

- `OwnStep::asked_of(adapter)` is std's table of what each adapter asks of
  what it holds as it takes each step: each source, and the step it asks.
- `through_a_mut_ref` says which steps std's `&mut I` passes on: `nth` and
  `nth_back`.
- `Reach` is a type parameter's answer. `Sure` says no but of the steps
  whose helper asks the iterator itself, `nth`, `last` and `count`; `May`
  says yes.

The table is the old predicates' answers, with one addition: a
`peekable()`'s `rfold`, its source's. `test/architecture.test.ts` ("which
own step an adapter reaches is the OwnStep catalog's alone") rejects a
per-step predicate. The 37 mutations of the old predicates mutate the
catalog's entries, each as it meant.

Left for later: `$iterator`'s options object, and the JS step table
`$given` reads.

## Amendment: what a slice's steps follow is the catalog's too (2026-11-04, #697)

Case ([0262](0262-when-rust-and-js-disagree.md)): R, results change. The JS
must drop what std drops, in std's order.

`recognition/slice_steps.rs` asks which adapters pass an `nth` or a fold on to
what they hold, so it can find a `Vec`'s `IntoIter` that drops what it passes
as one slice. It kept two lists of its own beside the catalog:

- `nth_back_rfolded_sources` held the adapters whose `rfold` asks an
  `nth_back`. It lacked the catalog's `peekable()`, so in generic code
  `src.take(3).peekable().rev()` was collected into a `Vec` by `next`
  where std folds it: under `--std-iterators` it dropped 4 before 5,
  where native Rust drops 5 before 4.
- `nth_drops_a_slice` held the adapters whose `nth` asks an `nth`. That
  was the catalog's `Nth` row.

Both lists are gone:

- `nth_back_folded_sources` follows `asked_sources` from a fold through each
  `fold`, `rfold` and `try_rfold` to the sources asked an `nth_back`.
- `nth_drops_a_slice` follows the `Nth` row.
- A chain's first side, which a chain's `nth` asks an `advance_by` of, is
  `nth_advanced_source`, in `own_steps.rs`. The crate's iterator never
  writes that step, so it isn't an `OwnStep`.

The old list followed a `skip`'s `rfold` into a `fuse()` and a `take`, so
the catalog now has std's `Fuse::try_rfold` and `Peekable::try_rfold`, each
its source's (`core/src/iter/adapters/fuse.rs`, `peekable.rs`). Its
`TryRfold` row had left both out.

`test/iterator-forms.test.ts`'s "a take's rev" adds a `peekable()`, a
`fuse()` under a `skip(1)` and a `peekable()` under one, collected into a
`String` of `char`s and of `String`s, and generic code's
`take(3).peekable().rev()` collected into a `Vec`; each is checked against
native Rust. A `String` of `String`s takes its first item by `next` and folds
the rest. On main, its `peekable()` under a `skip(1)` dropped 5 before 4,
where native Rust drops 4 before 5. `test/architecture-iterators.test.ts`
("which source an adapter's fold or nth asks is the OwnStep catalog's
alone") rejects a lowering function named for a fold or an `nth` that
names one of std's adapters outside `own_steps.rs`.
Mutations: `nth-back-of-a-take-unasked`, `nth-back-not-through-try-rfold`
and `nths-a-slice-not-through-adapters` (`lower/recognition/slice_steps.ts`);
`try-rfolds-not-through-fuse`, `try-rfolds-not-through-peekable`,
`rfolds-not-through-peekable`, `nths-its-own-no-peekable` and
`chain-nths-no-slice` (`lower/recognition/own_steps.ts`). The catalog's
`nths-its-own-no-enumerate` and `nths-its-own-no-fuse` also run the slice
case that the old list's mutations of those adapters ran.

## Amendment: a type parameter's own steps are its value's to say (2026-11-04, #621, #623, #637, #649)

Case ([0262](0262-when-rust-and-js-disagree.md)): R. Each changed
results, so Rust's behavior is kept.

A dictionary of a type that holds a type parameter, generic code's
`Map<I, F>` or `I` itself, said what `Reach::Sure` says: no `fold`, no
`rfold`. So std's compiled `Fuse`, `Peekable` and `Map` given it folded
the parameter by `next`, never calling the crate's own `fold` or `rfold`
its caller's value writes: `src.map(f).fuse().fold(..)` and
`src.peekable().rev().fold(..)` of the crate's iterator in generic code
(#621, #649), and a fuse of it dropped twice where the own step ran anyway
(#623, #637). A dictionary can't say which values a parameter is given, so
the answer is asked of the value at run time:

- **A type parameter's dictionaries say what `Reach::May` says**: `fold:
  $foldOf`, `rfold: $foldBack`, each std's helper, which asks the value
  itself (`iterator_dictionary`, `iterator_extension_dictionary`).
- **Whether a dictionary's step took the value, so generic code drops
  nothing of it after, is `$took(step, value)`**: none didn't; the crate's
  own or std's compiled step did; std's helper did where the value's own
  ran, the crate's (`ownFold`, `ownRfold`, `ownLast`, `ownCount`) or std's
  compiled struct's (`takes`). It was `IIterator.fold === undefined`, which
  a helper's presence made false of every value. `@rust-js/std`'s compiled
  bodies read `if (!$took(IIterator.fold, iter)) dropI?.(iter);`.
- **The helpers a dictionary may hold are one table, `$askedStep`**, which
  `$given` reads too: a dictionary of only those is the value's own steps.
- **A compiled struct stepped from its back is given std's compiled
  `rfold` whatever it holds**, as its `fold` is (ADR 0912): std's
  `Peekable::rfold` drops its source before it folds what it peeked at,
  where `$foldBack` of its `next_back` dropped it after. Like its `fold`,
  it takes the struct once the call ends, though it throws (`$closed`), so
  generic code's close after one whose source's `next_back` panicked drops
  nothing twice.

`own_steps_a_type_parameter_may_reach.rs` holds the shapes against native
Rust: a `map().fuse().fold(..)` and a `peekable().rev().fold(..)` in
generic code of the crate's iterator that writes its `fold` and `rfold`,
one that writes neither, and a `Vec`'s; a peekable's `rev()` that isn't
generic, and one generic code folds whose source's `next_back` panics;
and a `sum()` through a held struct's dictionary of a value generic code
was given, and of its `map()`.
Mutations: `type-parameter-fold-unsaid` and `type-parameter-rfold-unsaid`
(`lower/iterators/dictionaries.ts`), `knowing-rfold-only-its-owns`
(`lower/iterators/knowing.ts`), `dictionary-step-took-unasked`
(`lower/mir/iter.ts`), `took-helper-trusted` and `took-own-unasked`
(`runtime/iterator.ts`) and `std-iterator-unrfolded`
(`runtime/std_iterator.ts`); `own-steps-not-the-value`, `given-count-not-own`,
`given-boxed-steps-not-own` and `knowing-rfold-unasked` name their new
text, and `std-struct-fold-undictionaried` a compiled struct's JS, as a
type parameter's `fold` is `Reach::May`'s now.

## Amendment: a flatten stepped from its back folds as std's `FlattenCompat` does (2026-11-04, #621, #649)

Case ([0262](0262-when-rust-and-js-disagree.md)): R, results change, so
Rust's behavior is kept.

`$flattenEnds`, a flatten the crate steps from its back, had no `fold` and
no `rfold`, so std's defaults stepped it by `next` or `next_back`, and
neither its source's own `fold` or `rfold` nor an inner's was called.
`Folds(0, 3).map(pair).flatten().rfold(..)` never ran the crate's own
`rfold`, and its `rev().rfold(..)`, `Rev::rfold`, which is
`Flatten::fold`, never ran its own `fold`. It now gives both by
`$forward`, std's `FlattenCompat::fold` and `rfold`: the inner it's in at
that end, then its source's own fold of each item's inner, each inner's
own, then the inner at its other end. `OwnStep::asked_of` says a
flatten's and a flat map's `rfold` is its source's, as their `fold` is,
so the table is std's; no program's JS differs by it today, as each place
that asks it of a flatten asks the value at run time too, so no mutation
holds it.

`flatten_folds_reach_its_sources_own.rs` checks a flatten's and a flat
map's `rfold`, a `rev()`'s `fold` and `rfold`, generic code's, each
inner's own, and a closure that panics part-way, against native Rust in
both builds. In the matrix, #621's 14 cases, 123 of #649's and 20 of
#650's run as Rust does. Mutations: `flatten-ends-fold-unforwarded` and
`flatten-ends-rfold-unforwarded` (`runtime/back_steps.ts`);
`rfolds-not-through-peekable` names its line as it now reads.

## Amendment: the crate's own `nth_back` says it's its own, so a `rev()` of it is (2026-11-04, #649)

Case ([0262](0262-when-rust-and-js-disagree.md)): R, results change, so
Rust's behavior is kept.

Std's `Rev::nth` is its source's `nth_back`, and a `skip` and a `step_by`
step what they hold by `nth`. `$iterator`, generic code's JS iterator of a
value it was given, said `ownFold`, `ownNth`, `ownLast`, `ownRfold` and
`ownCount` of the crate's own steps, but never `ownNthBack`, and
`$closing` didn't carry it on. So a `rev()` of it never said its `nth`
was its own, and `src.rev().skip(2)` and `src.rev().step_by(2)` in
generic code stepped it by `next_back`, never calling the crate's own
`nth_back`. `$iterator` now says each of its own steps by one loop over
them, `$ownStep`'s names, and `$closing` carries `ownNthBack` as it
carries `ownNth`. A type parameter's `Reach::Sure` still says no of
`nth_back`, whose helper doesn't ask the value; the answer is the
value's, at run time.

`generic_rev_nths_its_own_back.rs` checks a `skip`'s collect and `last`
and a `step_by`'s collect and `count` of a reversed type parameter, the
crate's iterator that writes `nth_back` and a `Vec`'s, against native
Rust in both builds. The matrix takes off #649's last 72 cases, its
`rev.skip` and `rev.step_by` ones. Mutations:
`iterator-own-nth-back-unsaid` (`runtime/iterator.ts`) and
`closing-own-nth-back-unsaid` (`runtime/closing.ts`).
