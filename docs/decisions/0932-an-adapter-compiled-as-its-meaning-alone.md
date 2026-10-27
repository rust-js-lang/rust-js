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
