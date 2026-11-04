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

`test/architecture.test.ts` ("an iterator's steps are given by $forward
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
