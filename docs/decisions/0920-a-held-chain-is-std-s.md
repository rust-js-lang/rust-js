# 0920. A held `Chain` is std's, stepped from its back by its own compiled methods

Status: Accepted, under the user's goal of 2026-10-26 ("keep going until
all done, the choice is your recommendation"). The plan "rust-js: one
representation per iterator", its second phase. Extends
[0917](0917-a-held-step-by-is-std-s.md),
[0552](0552-an-into-iterator-held-as-it-is-is-given-as-it-is.md) and
[0525](0525-a-cloned-map-is-made-again.md).

Case: C ([0262](0262-when-rust-and-js-disagree.md)): results change. With
std's structs alone, a held chain stepped from its back threw `it.back is
not a function`, as two of the corpus's cases did.

## Context

Compiling std's `Chain` found four things, each general:

- std's `Chain::nth_back` calls `advance_back_by` of each side, which
  rust-js didn't lower.
- std's struct a function gives behind an `impl DoubleEndedIterator` was
  given its compiled back where `rev()` asks it, `$knowing(..)`, but not
  where `rfind`, `nth_back` or `rfold` do.
- An `IntoIterator` dictionary's `IntoIterIterator`, of a type parameter
  given as it is, `I: Iterator` made into itself, stepped what it makes
  as a JS iterator, not by the dictionary `I` was given.
- Whether a function holds what its `J: IntoIterator` makes, so `J` is
  given its dictionary, read its signature's types but not what an
  `impl Iterator` it gives hides, `Chain<I::IntoIter, J::IntoIter>`: a
  generic `chain` was refused once std's `Chain` held `J::IntoIter`.

## Decision

- **A held `chain()` is std's `Chain`**, with `next`, `size_hint`,
  `count`, `fold`, `try_fold`, `advance_by`, `nth`, `last`, `next_back`,
  `nth_back`, `rfold`, `clone` and `fmt` compiled. Not its `find`, whose
  closure takes the predicate, which the drop analysis refuses; its
  `try_fold` gives the same order but where the crate writes its own
  `find`.
- **`advance_back_by` in std's bodies is `$advanceBackBy`**, given as
  `$nthBack` is: its own, a `Vec`'s as one slice, else each `next_back`.
- **std's struct stepped from its back is given its compiled back**,
  `knowing_held`, in each back step as in `rev()`.
- **What an `I: Iterator` makes is stepped by `I`'s own dictionary**
  (`made_steps`), where it's given one.
- **What an `impl Trait` a function gives hides counts for what it holds**
  (`into_iter_held_as_it_is`).

## Consequences

- ADR 0908's `differs` is 23, from 24: `back_steps.rs` and
  `flatten_from_its_back.rs` run as Rust does with std's structs alone;
  `cloned_chains_and_cycles.rs` now differs there, a zip cloned apart
  cloning its part, std's `Chain`, as its maker made it. That goes with
  the zip's own compile, or with cloning apart, phase 4's.
- `generic_chain_of_into.rs` runs as native Rust does: a generic chain of
  two `IntoIterator`s, and a held chain's `rfind` and `nth_back`.
- A held chain reads as std's, `iterator_chain(..)` (`adapter_drops.js`,
  `returned_impl_ends.js`); in `adapter_drops.js` a temporary held for a
  throw is `value$6`, where tidy named it `countdown` after its type, as
  it no longer reaches tidy's suggestion: readability work left open.
- The readability baseline is blessed: `$iter` 85.1 to 86.5 in 10,000
  lines, 18 lines more, most a held chain's clone of its slices' sides,
  one dictionary each (`cloned_chains_and_cycles.js`, 36 to 48), where its
  maker named them once. std's struct reads heavier than the recognizers'
  chain it replaces until the plan's third phase gives back its rewrites.
- Mutations: `chain-not-held`, `advance-back-by-unlowered`,
  `advance-back-by-short`, `std-struct-back-unknown`,
  `made-steps-of-a-js-iterator`, `impl-trait-hides-held-into-iter`.
