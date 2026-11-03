# 0912. Std's struct folds by its own compiled `fold`, and a `&mut I` steps by what std's impl for it passes on

Status: Accepted, under the user's goal of 2026-10-26 ("keep going until
all done, the choice is your recommendation"). The second phase of the
plan "rust-js: one representation per iterator", its first part. Extends
[0871](0871-rev-is-stds.md), [0903](0903-a-flat-map-held-is-stds.md) and
[0908](0908-one-meaning-per-iterator.md).

Case: C ([0262](0262-when-rust-and-js-disagree.md)): results change. In
ADR 0908's build of std's structs alone, a `flat_map` over the crate's
iterator, fused and summed, stepped its `next` four times where Rust's
runs its `fold` once (`own_fold_and_nth.rs`); a `by_ref()` of a range or
of a type parameter taken by std's `Take` threw `it.next is not a
function`.

## Context

A held adapter's `fold` reached the crate's own `fold` through three
hand-written helpers, `$mapFold`, `$inspectFold` and `$flatMapFold`, one
per adapter, each std's `fold` written again in JS. Std's compiled `fold`
of each was there already. Where std's own body makes an adapter, as
`FlatMap` makes its `Map`, the struct's dictionary had no `fold`, so std's
`FlattenCompat::fold` stepped it by `next`.

Compiling `Skip` and `Take` from std (the plan's next step) found the rest:

- std's compiled `fold` takes its struct whole, as Rust moves it, and
  drops what it held there; a close after it, the caller's or `$closing`'s,
  dropped it again.
- `$stdIterator` gives the struct its methods as its own properties, and
  `Enumerate`'s field `count` is a method's name: its compiled `count`
  overwrote it.
- A `&mut I` was given a dictionary of a JS iterator, `{ next: $next }`,
  whatever `I` was, as std's impl for `&mut I` isn't a type parameter's.
- `$given` keeps a value as it is only for `{ next: $next }`, so a
  dictionary with `fold: $foldOf` wrapped std's struct and lost its own
  `advanceBy`.

## Decision

- **Std's struct folds by its compiled `fold`, its meaning.** Its steps'
  `fold` is std's compiled one (`compiled_fold`), and a dictionary of a
  type whose `fold` std compiles has `fold: $foldOf`, whatever its sources
  are (`compiled_fold_of`). `$mapFold`, `$inspectFold`, `$flatMapFold` and
  `Fuse`'s hand-written `fold` are gone; `Fuse`'s `try_fold`, `count` and
  `advance_by` stay hand-written until the plan's third phase.
- **What std's compiled methods take whole they mark `$closed`**, its
  steps' `takes`, `["fold", "count"]`, so no close after drops it again;
  `$closing` asks the same.
- **A field of std's iterator struct named as a method `$stdIterator`
  gives it is keyed apart**, `count$` (`field_key`), as the struct is that
  JS iterator too.
- **A dictionary of a value's own steps is the value**: `$next`, `$foldOf`,
  `$nth` and `$last` each call the value's, so `$given` gives it as it is.
- **A `&mut I`'s `Iterator` dictionary is what std's impl for it passes on
  to `I`**: `I`'s `next`, `size_hint` and `nth`. Not its `fold`, which is
  `I`'s `try_fold`, nor its `last`, std's of `next`, as `$lent` has it
  (`mut_ref_iterator_dictionary`).

Held `Skip` and `Take` stay the recognizers'. Compiled from std, a `take`
over a `skip` folds through `Take::try_fold`, generic over `R: Try`, which
rust-js can't lower yet; without it, std's struct steps `Skip`'s `next`,
`nth(n)`, where Rust's `fold` asks `nth(n - 1)` once, a wrong answer the
corpus doesn't cover. A generic `Try` is the plan's next step: `Take`,
`Skip`, `Chain` and `StepBy` fold through it.

## Consequences

- ADR 0908's `differs` is 32, from 33: `own_fold_and_nth.rs` runs as Rust
  does with std's structs alone.
- A dictionary of std's struct whose `fold` std compiles names `fold:
  $foldOf`, a `Fuse`'s and a `Map`'s (`held_peekable.js`,
  `step_by_compiled_source_nth.js`, 16 snapshots). The readability baseline
  is blessed: `Iterator.from` 15.3 to 15.4, the new case's generic
  `halves`; `$iter` 87.1 to 87.0.
- `fold_by.js` has three helpers fewer, 49 lines.
- `held_adapters_of_by_ref.rs` holds std's `Map`, `Fuse` and `Peekable`
  over a `by_ref()` of a range, of the crate's iterator and of a type
  parameter's, as native Rust does, none calling the crate's `fold`.
- Mutations: `compiled-fold-untaken`, `compiled-count-unlisted`,
  `std-iterator-takes-unmarked`, `closing-closed-again`,
  `std-struct-fold-undictionaried`, `own-steps-not-the-value`,
  `mut-ref-dictionary-whole`, `mut-ref-dictionary-unpeeled`,
  `std-field-key-shared`.

## Amendment: a compiled `last` takes its struct too (2026-11-03, #522)

Case ([0262](0262-when-rust-and-js-disagree.md)): R, results change, so
Rust's behavior is kept.

std's `last` of a `Skip`, a `Fuse`, a `Peekable`, a `Copied` and a `Chain`
is `self.iter.last()`: it takes the struct whole and moves its source into
its source's `last`. The compiled bodies never dropped that source, and
`last` wasn't in `takes`. So the caller's close after `last` dropped it.
That was right for a source whose `last` is std's, and a second drop where
the crate's own `last` (or `fold`) had already dropped what it held.
`Lasts(ds()).fuse().last()` printed `drop 1, drop 2` twice, in the plain
build too. The bodies didn't drop the source because `dropped_after`
(`lower/mir/iter.rs`) took `Option<I::Item>` to hold the `I` it consumed:
walking the projection `<I as Iterator>::Item` yields `I`.

Now:
- What a method gives holds what it consumed only outside a projection,
  `mut_refs::holds_outside_projections`, the rule `may_move_out` follows
  too. So each compiled `last` drops its source after, as its `count`
  does: `if (IIterator.fold === undefined && IIterator.last === undefined)
  dropI?.(iter)`, the crate's own `last` or `fold` having taken it
  otherwise.
- `last` is in `takes` wherever it's compiled, and `$stdIterator`'s
  `last` marks its struct closed, so no close after drops it again.
- Generic code given no dictionary, the crate's own generic functions,
  now drops what `last()`, `max()`, `sum()` and `for_each` consumed after
  them only where its JS iterator says the crate's own steps didn't take
  it: `if (!iter.ownFold && !iter.ownLast)`. Before, `last()` and `max()`
  of one with a destructor were refused, and `sum()` and `for_each` of one
  whose own `fold` dropped it dropped it twice.
- `$given` also treats the boxed steps, `$nextSome`, `$nthSome`,
  `$lastSome` and `$nextBackSome`, as the value's own where it's given
  `boxed`: unboxed again, they are the value's steps. Generic
  `it.skip(1).last()` of a JS array iterator, std's compiled `Skip`'s
  `nth(0)`, threw `it.nth is not a function` in the build of std's structs.

`last_takes_its_source.rs` holds the shapes against native Rust, in both
builds. `generic_own_last`, `skip_folded_by_consumers` and the other
matrices of #492-#496 and #505 now run in iterator-forms' `few`. Mutations:
`own-last-dropped-again`, `last-output-holds-its-iterator`,
`generic-own-steps-unasked` (`lower/mir/iter.ts`), `compiled-last-not-taken`
(`knowing.ts`), `std-iterator-last-not-taking` (`std_iterator.ts`) and
`given-boxed-steps-not-own` (`iterator.ts`).
