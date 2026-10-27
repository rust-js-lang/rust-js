# 0975. std's `Zip` is compiled, its impl picked by its sides' facts

Status: Accepted, under the user's goal of 2026-10-26 ("keep going until
all done, the choice is your recommendation"). The plan "rust-js: one
representation per iterator", its second phase. Extends
[0899](0899-a-flatten-held-is-stds.md)'s and
[0543](0543-a-held-peekable-keeps-what-it-peeked-at.md)'s facts that pick an impl, and
[0932](0932-an-adapter-compiled-as-its-meaning-alone.md)'s adapters compiled as
their meaning alone.

Case ([0262](0262-when-rust-and-js-disagree.md)): results change. std's
zip of two `TrustedRandomAccess` sides runs a longer side's closure less
often than its general impl does. One rule below,
`MAY_HAVE_SIDE_EFFECT` read as true where rustc can't evaluate it, is
case A: no observable difference.

## Context

`zip_from_back` was in the iterator forms' `differs`: with std's structs
alone, a zip's `rev()` was refused, as no zip was std's `Zip`. std picks
`Zip`'s behavior from its sides' traits, which a program can see
(ADR 0973):

- `ZipImpl` has three impls: general, `TrustedRandomAccessNoCoerce` (its
  `fold` by index), and `TrustedRandomAccess` (`next`, `nth`, `next_back`
  by index), each specializing the one before.
- `SpecFold` folds a zip of two `TrustedLen` sides only up to the
  shorter.
- `A::MAY_HAVE_SIDE_EFFECT`, a constant of each side, says whether the
  items it passes over are read.
- `__iterator_get_unchecked(i)` reads a side's item by index.

rust-js's facts picked between two impls, by one fact of one type.

## Decision

`Zip` is compiled as its meaning (`MEANING_ONLY`), with its `next`,
`size_hint`, `nth`, `fold`, `next_back` and `__iterator_get_unchecked`,
std's `zip`, and `TrustedRandomAccessNoCoerce::size`.

- **Facts pick from a chain of impls.** `FactSpecialization` lists each
  specializing impl that writes the method, the most specific first, with
  every fact that picks it, of every type: `RandomAccess` of both `A` and
  `B`. The call is a chain of conditionals. An impl that writes no method
  of its own leaves it to a more general one.
- **New facts.** `RandomAccess`, `RandomAccessNoCoerce` and `TrustedLen`
  pick impls only of a zip's private traits (`ZipImpl`, `SpecFold`,
  `SpecTrustedRandomAccess`). Elsewhere, as ADR 0899 decided, they are
  std's optimizations.
- **One table of fact traits.** `fact_trait` is the one table from a fact
  to its trait, which `specialized_on`, a fact's value, `is_trusted_len`
  and `zips_by_index` read.
- **Facts rustc can decide.** Of a type that holds a parameter, a fact is
  what rustc can say where it's asked: `slice::IterMut<'_, T>` is random
  access of any `T`, and no impl makes a `&mut I` one.
- **Facts of what a type holds.** A fact std's one impl for a type
  constructor asks only of its parameters is theirs (`held_facts`):
  `TrustedLen` of a `&mut I` is `I`'s. The specialization, the fact's
  value and the analysis all read this.
- **Reads by index.** `__iterator_get_unchecked` is `IterMethod::GetUnchecked`,
  a parameter's dictionary's `getUnchecked`. It is `$getUnchecked` of an
  array's `$iter`, `$rangeAt` of a range, and the compiled reader of
  `Map`, `Enumerate`, `Copied`, `Skip`, `Take`, `Fuse` and `Zip`. It is
  given only where `Zip` is compiled. A random-access fact of a type
  rust-js can't read by index is refused, never given. `$given` sets the
  reader aside, as it steps nothing.
- **`MAY_HAVE_SIDE_EFFECT`.** It is rustc's value; where rustc can't
  evaluate it, of a type only a caller knows, it is true. What it lets
  std's zip leave unread does nothing when read (a slice's item read and
  dropped, a back trimmed), so reading it gives the same results. That
  is case A, and it needs no fact.
- **Defaults that only panic.** A specialized-on default whose body never
  returns, std's "always specialized", is not one the facts pick: the
  least specific specialization takes its place. Its inlined
  `panic!("..")` is a raw pointer rust-js can't lower, and its callers'
  bounds rule it out. No stable program can make it run.
- **One list of steps.** The steps a compiled struct is made with are one
  list, `STRUCT_STEPS`, which `type_facts` reads too. It read four of
  eight, so a `fold`'s facts were missed; a step's own generics, `fold`'s
  `B` and `F`, are its own parameters there. Held `Fuse`s and `Flatten`s
  are now given the `Fused` fact their folds ask, where before they
  defaulted to "not fused", which gives the same results of a fused
  source (four snapshots).
- **Runtime checks.** An `Operand::RuntimeChecks` is false, as
  `std_coverage` reads `UbChecks` and as a release build's overflow
  wraps (ADR 0364).

## Consequences

`zip_from_back` and `result_given_as_items` (ADR 0976) run as Rust does
with std's structs alone. With its derived `clone` compiled too,
`cloned_chains_and_cycles` does, and with ADR 0977's `evidence_paths`,
the iterator forms' `differs` is empty: every corpus case runs as Rust
does with std's structs alone. A zip made and
consumed in place is still the recognizers' `$lazyZip` in either build
(ADR 0871). So #380, a zip whose `next` and `fold` differ in std itself,
waits on the in-place chain being std's structs, phase 4.

In the normal build a held zip's `next_back()`, and `rev()` of a zip of a
`map`, are still refused.

`test/corpus/held_zips.rs` checks held zips of a `map`, an `enumerate`,
a `skip`, a `take`, a `fuse` and a nested zip, by `next()` and `nth`, in
both builds. The mutations are:

- `zip-specialization-unfacted`
- `never-returning-default-picked`
- `held-facts-unreduced`
- `given-reader-a-step`
- `struct-steps-own-params-unfilled`
- `zip-clone-uncompiled`
