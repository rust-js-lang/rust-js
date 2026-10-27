# 0913. Std's generic `R: Try` is a dictionary, so a held `Skip` or `Take` is std's, and std's adapters fold and try as std's do

Status: Accepted, under the user's goal of 2026-10-26 ("keep going until
all done, the choice is your recommendation"). The plan "rust-js: one
representation per iterator", its second phase, and the start of its
third: the hand-written helpers std's compiled bodies now replace are
gone. Extends [0912](0912-a-std-struct-folds-by-its-own-compiled-fold.md),
[0871](0871-rev-is-stds.md) and [0177](0177-nonzero.md); amends
[0584](0584-a-transparent-struct-is-its-field.md) (`NeverShortCircuit`).

Case: C ([0262](0262-when-rust-and-js-disagree.md)): results change. A
held `take()` over a held `skip()`, folded after its `size_hint`, asked
the crate's `nth(2)` where Rust asks `nth(1)`; a held
`repeat(Loud).take(n)` given to `extend` never dropped its `Loud`; and
ADR 0908's build of std's structs alone differed on 31 corpus cases.

## Context

ADR 0912 held back compiling `Skip` and `Take`: std's `Take::fold` is
`self.try_fold(init, NeverShortCircuit::wrap_mut_2(f)).0`, and
`Take::try_fold` is generic over `R: Try`, which rust-js didn't lower. So
are std's default `advance_by` and `nth`, by `try_fold`, and each
adapter's own `try_fold`, `Map`'s, `Enumerate`'s, `Fuse`'s and the rest:
without them, std's struct stepped by `next` where Rust asks a `Skip`'s
`nth(n - 1)` once, and `Fuse`'s and `Peekable`'s `try_fold`, `fold`,
`count` and `advance_by` were written again in JS, seven helpers.

## Decision

- **`Try` and its supertrait `FromResidual` have dictionaries**,
  `{ fromOutput, branch, FromResidual }`: a type parameter's is given, as
  any bound's; a `ControlFlow`'s is `$controlFlowTry`, a
  `NeverShortCircuit`'s `$neverShortCircuitTry`. `?` of an `Option`, a
  `Result` or a `fmt::Result` stays as it is written
  (`tried_as_written`); of another `Try`, a call through its dictionary.
- **`try_fold` into another `Try` is `$tryFoldInto(it, init, f, tried)`**,
  the JS iterator's `tryFold` in `R`'s terms.
- **A JS iterator's `tryFold(init, f, broke)` calls std's compiled
  `try_fold` with `$tryOf(broke)`**: what it's made so far is its output,
  and what `broke` stops it its residual, as it is
  (`compiled_try_fold`). A call gives a bound's dictionary it makes
  itself through `evidence_args_given`.
- **A held `skip()` or `take()` is std's `Skip` or `Take`**, with
  `fold`, `try_fold`, `count` and `advance_by`; and std's `Map`,
  `Enumerate`, `Inspect`, `Fuse` and `Peekable` fold, try, count and
  advance by their own compiled methods. `Flatten`'s and `FlatMap`'s
  `try_fold` and `advance_by` stay out: they reach
  `unreachable_unchecked` and nested functions. `Rev`'s `advance_by`
  stays `$revAdvanceBy`.
- **What std's bodies need, each general**: `I::advance_by(n)` is
  `$advanceBy` (a `&mut I`'s is its `I`'s, `$lent`); a number
  transmuted to an `Option<NonZero<_>>`, by a call or a MIR cast, is
  `NonZero::new` (`non_zero_transmute`); a `NeverShortCircuit` is its
  value, as `AssertUnwindSafe` is; a `&mut` of a number a closure
  captured as its place stays that place where optimized MIR copies it
  into a local (`captured_place`); a trait std keeps private,
  `SpecTake`, has its impls' methods compiled.
- **Std's struct is dropped the one way, by closing it**
  (`dropped_by_closing`), once: one that's std's wherever it's made, a
  `Peekable` or a `Fuse`, as an adapter held as std's and made in place
  as the recognizers' chain is already, so a `Peekable` its compiled `fold` took
  isn't dropped again by its fields. **One given whole to `extend` is
  dropped after**, as `extend` drops what it's given. `$closing` passes
  on its iterator's own `count`. **`count()` of a type parameter, or of
  std's struct whose `count` is compiled, is `$count`**, its own.
- **Retired**: `$fuseFold`, `$fuseTryFold`, `$fuseCount`,
  `$fuseAdvanceBy`, `$peekableFold`, `$peekableTryFold`,
  `$peekableCount`, and `$foldBy`, `$countOf` and `$tryFoldBy`, which
  only they used; the lowering that made them; and `is_take`.

## Consequences

- ADR 0908's `differs` is 28, from 31 (33 at the plan's start):
  `held_adapters_by_ref`, `own_nth_through_adapters`,
  `adapters_own_nth_fold` and `step_by_compiled_source_nth` run as Rust
  does with std's structs alone. The rest are refused (`Chain`'s,
  `StepBy`'s, `Copied`'s and `Zip`'s backs) or std's struct steps a JS
  iterator the recognizers made (`it.back is not a function`).
- `held_skip_take.rs` holds `skip` and `take` folded, tried, counted
  and extended, as native Rust does.
- A held `skip()` or `take()` reads as std's, `iterator_take(..)`; a
  `Peekable` or a `Fuse` dropped on a panic is `finally { it.return() }`
  where it was each field's drop in each `catch`; a `count()` of what
  may count itself is `$count(it)`. 36 snapshots, about 490 lines fewer;
  the readability baseline is blessed, 352 lines fewer in all, `$iter`
  86.3 and `Iterator.from` 15.3 in 10,000 lines, from 85.9 and 15.2, of
  the new case's sources over fewer lines.
- `fold_by.js` has four functions, from fourteen.
- Mutations: `control-flow-try-never-breaks`, `tried-of-never-breaks`,
  `try-fold-into-breaks-on-continue`, `compiled-try-fold-unlisted`,
  `captured-place-copied-as-box`, `other-try-as-written`,
  `param-count-reduced`, `never-short-circuit-a-tuple`,
  `control-flow-try-never-short-circuits`, `given-evidence-ignored`,
  `extended-std-struct-undropped`, `std-struct-dropped-by-parts`,
  `closing-count-unforwarded`, `take-not-held`.

## Amendment: `$closing` passes on no `count`

`$closing` passed on its iterator's own `count`, a compiled `Take`'s.
Since ADR 0925 a compiled struct drops what it holds itself and is never
wrapped in `$closing` (`closing_drop`), and what is wrapped, the crate's
own iterator, a collection's `IntoIter` or a type parameter's, has no
`count` of its own: an adapter given to a generic function whose drop it
doesn't take is refused. No case of the corpus, either build, reached it,
and `closing-count-unforwarded` survived every run, so the code went, and
its mutation with it.
