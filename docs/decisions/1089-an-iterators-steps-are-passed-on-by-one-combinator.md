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
