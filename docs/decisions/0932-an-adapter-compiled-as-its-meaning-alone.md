# 0932. An adapter may be compiled as its meaning alone: std's struct in the build of std's structs, the recognizers' wherever the normal build makes it

Status: Accepted, under the user's goal of 2026-10-26 ("keep going until
all done, the choice is your recommendation"). The plan "rust-js: one
representation per iterator", its second phase. Extends
[0908](0908-one-meaning-per-iterator.md) and
[0917](0917-a-held-step-by-is-std-s.md).

Case: C ([0262](0262-when-rust-and-js-disagree.md)): results change.
With std's structs alone, a held `map()` stepped from its back through
the recognizers' `filter`, `$filterEnds`, threw `it.back is not a
function`, std's `Map` made knowing no back; as did a `rev()` of a lazy
chain of one, and two clones.

## Context

The build of std's structs alone (ADR 0908) checks the meaning each
rewrite must agree with; an adapter std doesn't compile is the
recognizers' there too, so a case mixing one with std's structs isn't
checked. Compiling std's `Filter`, and holding it as std's struct as a
`Map` is (ADR 0871), was tried first: it changed 23 cases' JS, 479 lines
in, 120 out; made a function's `(0..n).filter(p)` it gives back
`iterator_filter({ start: 0, end: n }, p, { next: $rangeNext })` where it
was `$lazyRange(0, n).filter(p)`; and its clone of a `copied()` of a
`map()` made in place met that `map()` as std's struct. The normal build's
answers didn't change: its held filters were right.

## Decision

- **`MEANING_ONLY` lists the adapters compiled as their meaning alone**:
  each is in `COMPILED_STD`, `COMPILED_STD_PROVIDED` and `HELD_COMPILED`,
  compiled into `@rust-js/std` and std's struct in the build of std's
  structs alone, as the others are, and in any other build what it was
  before it was compiled, the recognizers', wherever the crate makes one.
- **One question answers it** (`compiled_for_build`, of a type name, and
  `not_compiled_for_build`, of a std item, an impl of it, its method or its
  maker), asked by every catalog query and by what the library contract
  says of std's items, `item`, `in_library` and `has_impl`, of a session
  flag `main` sets once: std's own bodies' build, `--std-bodies`, and the
  differential build, `--std-iterators`, build std's structs as their
  meaning (`build_std_structs_as_meaning`).
- **`Filter` is the first**, its `next`, `size_hint`, `count`, `fold`,
  `try_fold`, `next_back`, `rfold`, `try_rfold`, `clone` and `fmt`
  compiled. An adapter leaves `MEANING_ONLY` when it's held as std's
  struct, its readability weighed then (phase three's).
- **What finds an item with a destructor, `$findDropping`, finds it by
  the iterator's `tryFold` where it has one**, as std's `find` is its
  `try_fold`, and `$Steps`' `find` is: std's `Filter::next`, `self.iter.find(p)`,
  of a `skip()`, asks its source `nth(n - 1)`.
- **`Filter::count`'s `SpecAssumeCount` is an optimization by type**
  (`OPTIMIZING_BY_TYPE`): its `TrustedLen` impl assumes what the general
  one checks, `upper - count`, which a `TrustedLen`'s upper bound never
  fails.

## Consequences

- ADR 0908's `differs` is 11, from 15: `back_stepped_adapter_sources.rs`,
  `lazy_iterator_clone.rs`, `rev_of_lazy_chains.rs` and `std_map.rs` run
  as native Rust does with std's structs alone; `rev_of_lazy_chains.rs`,
  of the suite's few, checks it on each run.
- The normal build's JS doesn't change: every case's snapshot is as it was.
- The rest of std's adapters, `FilterMap`, `SkipWhile`, `TakeWhile`,
  `MapWhile`, `Scan`, `Cycle`, `Zip`, can be compiled as their meaning the
  same way, before each is held.
- Mutations: `meaning-only-made-everywhere`, `meaning-only-library-items-seen`,
  `filter-meaning-unmade`, `std-bodies-not-the-meaning`,
  `assume-count-specialized`, `find-dropping-by-next`; `instance-imported`
  follows its line; `skip_try_fold.rs` joins the suite's few.

## Amendment: `$findDropping` is its `tryFold`'s alone

`$findDropping` stepped an iterator with no `tryFold` by `next`. What it's
given is always the `&mut` `find` takes, `$lent`'s, a `$Steps`, which has
one; no case reached the other way, and `find-dropping-keeps-rejected`,
which took its drop away, survived every run. It went, with
`find-dropping-by-next`, which made every iterator take it; the drop of
what `tryFold` rejects is `find-dropping-keeps-rejected`'s now.

## Amendment (2026-11-11, #1401): the catalogue is asked of a type

The catalogue of std's compiled structs answered by an adapter's name
(`compiled_for_build(type_name)`), so every value of `Filter` had one
answer. A `{:?}` of one `Filter`, which only std's struct can show, couldn't
make that one std's struct while every other stayed the recognizers' (the
user's decision (a), 2026-11-11). So each question the catalogue answers of
a value now takes its type:

- `makes_held_compiled` and `sized_in_place` take the type the maker
  makes;
- `is_compiled_std_type` and `is_held_compiled` take the type;
- `compiled_instance`, a call's, takes its arguments, so an impl's method
  of an adapter is asked of the type it's of.

They share one private answer, `compiled_for(ty)`. `is_held_compiled_def`,
the name's answer for every value, is gone. A kind's own question, which
adapters the crate's types hold (`explicit_adapters`), asks the kind's
identity type, which no value's answer is. The queries that answer whether a
body is in `@rust-js/std` stay a definition's (`is_compiled_std`, the
library contract's `not_compiled_for_build`). `in_place_adapters` records
only the spans of makers whose made type is held, so `adapter_in_place`
asks the span alone.

No behaviour changes yet: `compiled_for` still answers by the name, so the
corpus's snapshots, in both builds, and `@rust-js/std` are byte-identical.
test/architecture-iterators.test.ts holds the queries type-keyed, and a
query by a definition alone from coming back.

## Amendment (2026-11-11, #1401): a type whose `{:?}` is asked is std's struct

Case ([0262](0262-when-rust-and-js-disagree.md)): none for results;
refusals lifted. In the normal build, `{:?}` of a `MEANING_ONLY` adapter was
refused: `Filter`, `FilterMap`, `TakeWhile`, `SkipWhile`, `MapWhile`, `Scan`,
`Copied` or `Cloned`. A JS iterator, `range.filter(p)`, keeps nothing to
show. That refused itertools' test_core.rs at `exactly_one().unwrap()`,
whose `ExactlyOneError<Filter<..>>: Debug` holds one. The user decided (a):
such a type is std's struct where it's held, and every other value of the
adapter stays the recognizers'.

`note_debug_shown` notes, once a session and before the catalogue is
asked, each `MEANING_ONLY` type whose `{:?}` the crate's bodies ask. That
covers every type a `Debug` clause of what they call names, `{:?}`'s own
`Argument::new_debug` among them, and each adapter inside it, so the one
in `ExactlyOneError<Filter<..>>` counts too. A closure's type is its site's,
so this picks one `filter()` out of many.

The set keys a type by its address interned in the session, regions
erased. That names a type in this `TyCtxt` alone, so it lives in the
session's caches, dropped with the session, never written to disk.

`compiled_for(ty)` answers yes of a type in the set. Several answers follow
that one:
- `@rust-js/std`'s bodies of an adapter one of whose types is in the set are
  this crate's to call (`called_for_build`, `not_compiled_for_build`).
- A maker's call, `filter`'s, is compiled only for the type it makes
  (`compiled_instance`).
- A std impl is the crate's own iterator's only for a type compiled for
  this build (`user_impl`).
- A resolved std method is the crate's only for its type (`resolves_to_crate`).

So in one crate `(0..10).filter(p)` shown is `iterator_filter({ start: 0,
end: 10 }, p, ..)` with `filterDebug_fmt`, and another `filter()` is
`$lazyRange(0, 10).filter(p)`. Held by corpus case
`debug_of_adapters_compiled_as_their_meaning.rs`: each adapter's text, a
generic wrapper's derived `Debug` of a filter stepped part way, and two
filters nothing shows, against native Rust in both builds. Mutations
`debug-shown-unnoted`, `debug-shown-type-not-compiled`,
`debug-shown-maker-of-any-type` and `std-impl-of-any-type-user`. The
package `@rust-js/std` doesn't change: the meaning build already compiles
each adapter's `fmt`.

## Amendment (2026-11-13, #1458): `RepeatN`, and std's free functions in one table

`RepeatN` is compiled as its meaning alone. In the build of std's bodies,
`repeat_n(x, n)` is std's compiled `repeat_n`, which makes std's
`RepeatN`. Every other build makes the runtime's `$repeatingN(x, n)`, as
before, so its output is byte-identical: no readability change.

std's free functions compiled are one table, `COMPILED_STD_FNS` (module,
name, and the struct it makes where a build compiles it alone). It holds
io's `chain` and `take` (#1156), which `COMPILED_STD_IO_FNS` held, and
`iter`'s `repeat_n`. A free maker of a meaning-only struct isn't called
outside a build of its meaning (`not_compiled_for_build`), as a
meaning-only adapter's `Iterator` maker isn't.

Each of `RepeatN`'s methods std writes is compiled: `next`, `size_hint`,
`advance_by`, `last`, `count`, `len`, `next_back`, `advance_back_by`,
`nth_back`, `rfold`, `try_rfold`, `clone` and `Debug`'s `fmt`. Its drop
needed std's maker to be given the drop of what it holds (ADR 0561,
amended).

`Repeat` stays the runtime's `$repeating` in every build (#1569): its
`last` and `count`, which only panic "iterator is infinite", make a
`fmt::Arguments` of a literal, whose inlined `Arguments::from_str`
transmutes a pointer std's build can't lower, and a struct compiled
without them would loop forever where Rust panics.

`test/corpus/repeat_n_drops_what_it_holds.rs` holds an element with a
destructor through each of those, and a `zip`, made with no items too,
and advanced past its end, against native Rust in both builds;
`test/iterator-forms.test.ts` runs it in the build of std's bodies. The
twin net, over the conformance and consumers tables, pits `$repeatingN`
against std's `RepeatN`. Mutations: `std-maker-given-no-drop`
(`lower/analysis/drops.ts`), `repeat-n-maker-uncompiled`,
`repeat-n-maker-called-plain` and `repeat-n-compiled-in-every-build`
(`lower/recognition/compiled_std.ts`); `io-chain-fn-not-compiled`
re-anchored.

The plain build refuses `clone()` of a `RepeatN`, as it did before, and
the case leaves it out; the build of std's bodies compiles it.
