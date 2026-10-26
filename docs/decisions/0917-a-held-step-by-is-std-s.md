# 0917. A held `StepBy` is std's, a precondition check never runs, and a std struct is the JS iterator it is where it's first one

Status: Accepted, under the user's goal of 2026-10-26 ("keep going until
all done, the choice is your recommendation"). The plan "rust-js: one
representation per iterator", its second phase. Extends
[0913](0913-std-s-generic-try-is-a-dictionary.md),
[0899](0899-a-flatten-held-is-stds.md) and
[0364](0364-mir-lowering.md).

Case: C ([0262](0262-when-rust-and-js-disagree.md)): results change. A
`rev()` of a held `step_by()` was refused, the `DoubleEndedIterator` of a
`StepBy`, as was its `nth_back`.

## Context

Compiling std's `StepBy` found four things, each general:

- std's `NonZero::new_unchecked` asserts its precondition through
  `assert_unsafe_precondition!`: a block that assumes `UbChecks` and calls
  a nested `precondition_check`. rust-js reads `UbChecks` as off (ADR
  0364), but discovery of what to compile walked every block, and lowering
  lowered that one.
- `StepBy` is specialized for each integer range: `SpecRangeSetup::setup`
  counts what's left into the range's `end`, and `StepByImpl` and
  `StepByBackImpl` step by that count. The general impls step the same
  items, hints and counts. Only `Debug` shows the changed range.
- `StepBy::new`, which calls `setup`, is `impl<I> StepBy<I>`: no bound
  makes `StepBy<I>` an iterator there, so its JS-iterator steps can't be
  given where the struct is made.
- `StepBy::fold` folds `iter::from_fn(..)`, which rustc inlines into
  `FromFn(f)`: a one-item array to rust-js, not the iterator `from_fn` is.

## Decision

- **A held `step_by()` is std's `StepBy`**, with `next`, `size_hint`,
  `nth`, `fold`, `try_fold`, `next_back`, `nth_back`, `rfold` and `clone`
  compiled; not its `Debug`, which would show `setup`'s range.
- **A precondition check never runs**: a block that assumes `UbChecks`,
  or a test of `check_language_ub()`, which reads it, is left out of what
  a body runs (`std_coverage::reachable`), in finding what to compile and
  in lowering (`assumes_ub_checks`), the one rule.
- **`StepBy`'s specializations by type are optimizations**
  (`OPTIMIZING_BY_TYPE`): the general impls are called, all of them.
- **A std struct is the JS iterator it is where it's first one**: made
  where it's no iterator, in `StepBy::new`, it's the struct, and the call
  that gives it to a body where it's one, `step_by`'s, makes it
  `$stdIterator(..)` (`makes_no_iterator`), that body given its source's
  dictionary as though it made it (`body_aggregates`).
- **`FromFn(f)` is `$fromFn(f)`**, as `iter::from_fn(f)` is.

## Consequences

- ADR 0908's `differs` is 26, from 28: `consumer_drops_what_is_left.rs`
  and `step_by_nth_back.rs` run as native Rust does with std's structs
  alone. `rev_step_by.rs` is refused for the `ExactSizeIterator` of a
  `RangeInclusive<u8>`, another gap.
- `adapters_own_nth_fold.js`: a fused `step_by()` is
  `iterator_stepBy(counted(), 2, { .. })`, where it was `$stepBy($iterator(..), 2)`.
- The mark `mir_call_values` gave what a call made in place, for a
  `map()` a closure gives `flat_map` (#260), is gone: no case's JS depends
  on it, `flatten_of_iterators_from_back.rs`, its case, among them, and its
  mutation, `in-place-made-unmarked`, survived on main too. What's made in
  place is marked where the recognizers make it.
- Mutations: `ub-checks-assumed-reached`,
  `step-by-specializations-unoptimizing`, `no-iterator-made-unwrapped`,
  `no-iterator-made-unstepped`, `from-fn-a-tuple`, `step-by-not-held`.
