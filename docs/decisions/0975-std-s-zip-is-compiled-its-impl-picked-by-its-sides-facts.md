# 0975. std's `Zip` is compiled, its impl picked by its sides' facts

Status: Accepted, under the user's goal of 2026-10-26 ("keep going until
all done, the choice is your recommendation"). The plan "rust-js: one
representation per iterator", its second phase. Extends
[0899](0899-a-flatten-held-is-stds.md)'s and
[0543](0543-a-held-peekable-keeps-what-it-peeked-at.md)'s facts that pick an impl, and
[0932](0932-an-adapter-compiled-as-its-meaning-alone.md)'s adapters compiled as
their meaning alone.
Amended by [0997](0997-a-zip-that-folds-apart-is-stds-struct.md): `Zip` is
std's struct in every build where it's held, or folds apart; a fact
specialization picks where rustc resolves its default.

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
- **Reads by index.** `__iterator_get_unchecked` is
  `IterMethod::GetUnchecked`, `$getUnchecked(it, i, dictionary)`. It reads
  std's compiled adapter by the reader its dictionary carries,
  `getUnchecked`, the compiled reader of `Map`, `Enumerate`, `Copied`,
  `Cloned` (ADR 0938, amended, #727), `Skip`, `Take`, `Fuse` or `Zip`. An array's `$iter` and a range need no
  entry: it reads them as they are. Only compiled adapters' dictionaries
  carry the reader, and only where `Zip` is compiled (a first form gave
  every slice's dictionary an entry, 1,500 lines of the corpus with std's
  structs). A compiled adapter is read by its own reader before its
  recognizers' form: a `copied()` of a slice, an array's `$iter` in the
  normal build, is std's struct here. A random-access fact of a type
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
- `index-reader-array-first`

## Amendment: a fact no types given its parameters can make hold is known here (2026-11-11, #1385)

itertools' probe, past #1370, was refused at "whether it's a
`TrustedRandomAccessNoCoerce` of `vec::IntoIter<MultiProductIter<I>>`,
which no caller gives", `MultiProduct::last`'s zip. `trait_holds` took any
type holding a type parameter, whose head has an impl of the trait, for one
only its callers can answer. But `vec::IntoIter<T>`'s impl asks `T: Copy`,
and no impl makes `MultiProductIter<_>` `Copy`, whatever `I` is: it never
holds. `may_hold_for_some` (`generic_facts.rs`, the catalogue's owner)
asks rustc whether the trait may hold of the type with each type parameter,
and each const parameter, an unknown; where it can't, the fact is `false`,
known here, asked of no caller. Of `vec::IntoIter<T>` it may, so it stays
its callers', as before.

Held by test/crates/pairs/library_zip_of_what_is_never_random_access (a
library's zip of a `Vec` of what's never `Copy` with a slice's iterator,
compiled by its consumer, given numbers and items that drop) against native
Rust, refused without it, and the mutation `never-holding-fact-asked`.
itertools' probe now stops at the same zip's other side,
`vec::IntoIter<<I as Iterator>::Item>`, which may be `Copy`: a fact of a
type built of a slot that `Iterator::zip`'s manifest asks, which no caller
of a library's function compiled here is asked (`asked_of_caller` gives no
ask of it), a gap of its own.

## Amendment: a built type's zip fact is its callers', relayed (2026-11-11, #1389)

Past #1385, itertools' probe was refused at the same zip's other side,
`vec::IntoIter<<I as Iterator>::Item>`, whose item may be `Copy`. Two
things kept it from its callers. `held_bounds`, which takes a fact apart
through std's one impl of the trait for its type constructor, kept only
that impl's bounds of the same trait: `vec::IntoIter<T>`'s
`TrustedRandomAccessNoCoerce` asks `T: NonDrop`, another trait, so it gave
no facts at all, as if none were needed. An impl that asks its parameters
another trait now has the fact asked of the whole type. And `built_of`
relayed only a built type's layout and sort facts to a function's callers:
a zip's trait facts, `TrustedRandomAccess`, `TrustedRandomAccessNoCoerce`
and `TrustedLen`, of a type rustc can't answer here (`trait_holds`'s
`None`), are its callers' too, so a library's function a consumer compiles
is given them as a built slot, and its caller answers of its own types,
`vec::IntoIter<u32>`'s yes, `vec::IntoIter<Loud>`'s no.

Held by test/crates/pairs/library_zip_of_a_vec_of_a_slots_items (a
library's zip of a `Vec` of what's never `Copy` with a `Vec` of a type
parameter's items, compiled by its consumer, of numbers and of items that
drop) against native Rust, and the mutations
`other-trait-bound-taken-for-none` and `built-trait-fact-unrelayed`.
itertools' probe now compiles whole, nothing refused, and its JS fails at
run time in `chunk_by`, `Groupbylazy.new$ is not a function` (#1390).

## Amendment: a side's reader by index is there exactly where its fact may be true (2026-11-16, #1982)

Take generic code's `src.cloned().zip(10..13)`, with `src` an `I` that
its caller makes a slice's, moved through generic code so it's std's
struct. std's zip of it takes the index path by the caller's facts,
`IRandomAccess` of `I`. Its `Iterator` dictionary had a reader by index,
`getUnchecked`, only where the side is `TrustedRandomAccess` where it's
compiled (`implements_trait`). `Cloned<I>` is that only where `I` is, and
only the caller can say whether `I` is. So std's zip read the struct as a
range, `start + i`, and each pair's first item was `NaN` where native
Rust pairs the source's. That was the std build's wrong answer, found by
the copy matrix.

**The one rule: the reader is present exactly where the side's
`TrustedRandomAccessNoCoerce` fact, by which std's zip takes the index
path, may be `true`.** `trait_fact_may_hold` in
`src/lower/std_types/number/generic.rs` answers that, beside
`type_fact_value`, which makes the fact. For a type parameter, the answer
is whether its caller gives the fact (`IRandomAccess`); rustc takes a bare
parameter for no. For a type whose fact is what it holds' (`held_facts`:
`Cloned<I>`'s is `I`'s, a `Zip`'s is each side's), every held part's fact
must be able to hold. Otherwise it's what rustc says where it's compiled.

A fact no caller gives is never true, so std's own generic bodies, which
are given none of these facts, get no reader. Building one there would
step a parameter nothing gives (std's package refused it). A true fact
never goes without its reader: `type_fact_value` already refuses where
std's zip would read by index what rust-js can't read so.

Case ([0262](0262-when-rust-and-js-disagree.md)): results change, so
Rust's behavior is kept. Held by the corpus case
`zip_of_a_generic_sources_copies_moved_through_generic_code` against
native Rust in both builds, one of the iterator forms' few. Its rows, each
moved through generic code:
- a `cloned()` and a `copied()` zipped with a range;
- that zip zipped again;
- an `enumerate()` of a `copied()` zipped;
- a range zipped with a `copied()`, on the right.

Also held by the copy matrix's `cloned.map.zip`, `copied.map.zip` and
`copied.enumerate.zip` rows, and the mutations
`index-reader-only-where-generic-code-knows` and
`random-access-of-a-param-taken-here`.

## Amendment (2026-11-22, #2252): `Zip`'s `Debug` is std's compiled body, its `ZipFmt` picked by the same facts

`{:?}` of a `Zip` was refused, and then of two random-access sides it was
`Zip` by a type test in the lowering (`display/debug.rs`): a second owner
of `ZipFmt`'s choice beside std's own. Now `Zip`'s `Debug` is compiled into
`@rust-js/std` like its other bodies, and its call of `ZipFmt::fmt` is
picked by the sides' `TrustedRandomAccessNoCoerce` facts, as `ZipImpl`'s
are. That gives `Zip` alone of slices' iterators, ranges, a `map` of one or
a `Vec`'s, and `Zip { a: .., b: .. }` of any other, each side as stepping
leaves it, through generic code too. The type test is gone. Four changes
made it, each to its owner:

- `ZipFmt` is in `specialized_on`'s table of what a fact picks.
- A writer's call of a method of a trait std keeps private is a trait's
  call (`recognition::write_call`), whose impl is picked as any is. Only
  `@rust-js/std`'s bodies make one.
- A writer's call that facts pick arms each impl's writer in the branch
  its facts pick (`armed_branches`). `SinkWrite::If`, the one form of a
  conditional write, gains its `else`. Its options are as the impl's
  writer takes them, not the trait's method, which nothing compiles.
- An impl is compiled here where a method of it is
  (`traits::calls::compiled_here`). `@rust-js/std`'s build read what a std
  impl's methods ask of its parameters, the facts its dictionary takes,
  from the contract its previous build wrote, so a newly compiled impl's
  dictionary could never take one.

The `Zip` of a slice's `Chunks`, `v.chunks(1).zip(v.iter())`, which
printed `Zip` by type, asks `Chunks: Debug` now, as std's `impl<A: Debug,
B: Debug> Debug for Zip<A, B>` does. That landed first (ADR 0335,
amended), so no program that printed starts being refused. Held by
`debug_of_a_zip.rs` against native, and the whole corpus and crate corpus
compiled with it.
