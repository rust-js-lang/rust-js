# 1176. An in-place form answers the std type it stands for

Status: Accepted (2026-11-20, the lead's decision for #2112's Change A,
step 3, phase 1). Amends [0515](0515-map-while-and-kept-somes.md) and
[1092](1092-the-iterator-protocol-is-one-contract.md).

Case ([0262](0262-when-rust-and-js-disagree.md)): R. Which of the crate's
steps std calls is observable, so a form must call exactly those.

## Context

The recognizers make several std adapters in place as one runtime method:
`$Steps.filter` stands for `Filter`, and also for the `Some`s of a
`filter_map`, a flatten of `Option`s and a `flat_map` to them,
`src.map(f).filter((item) => item != null)` (ADR 0515). The runtime net
(test/iterator-steps.test.ts) held each runtime form to the std adapter
it's named for alone, so `$Steps.filter`'s own `count`, std's
`Filter::count`, which asks its source's `size_hint` first, was right for
`Filter` and wrong for the other three, whose `count` is their fold.

Consumers that know the type chose around it (#2197's fix: the count
where it's made asks `counts_by_its_own`). Generic code can't: given the
form, it counts by `$count`, the form's own (#2199). So a crate-wide fact
that makes a crate iterator knowing its hint, and its adapters in place,
asked the crate's own `size_hint` where std never does, the fourth such
wrong answer (#2157, #2193, #2194, #2197).

## Decision

**A form made in place answers the catalog row of the std type it stands
for, for every source, so no consumer has to choose around it.** The
first such answer is the `Some`s:

- **`$somes(it)`**, a runtime form: a filter of what's there,
  `$filterEnds`' where `it` knows its back, whose own `count` is its fold,
  std's `FilterMap`'s and `Flatten`'s; of a JS iterator that knows no
  back, JS's own `filter`, which has no `count`.
- **It's made where its source's `size_hint` may run the crate's code, by
  its type** (`hint_reaches_own_where`, of a type parameter's and what a
  `dyn` holds too). That's true of every value of the type alike, whatever
  form a crate-wide fact made it, so no fact chooses the form. Elsewhere
  the `Some`s stay `filter((item) => item != null)`, as JS writes them:
  asking a hint that runs nothing changes nothing.
- **Generic code's sources** (a type parameter's, a dictionary's) are
  `$somes`, as the site can't know what a caller gives: the readable
  filter could give a wrong answer there, and correctness decides (the
  lead, 2026-11-20).
- **The net holds it**: `$somes` of a map, and of a map that knows its
  back, run against `FilterMap`'s row of std's catalog.

Phases 2 and 3 (#2112): one owner names each in-place form's std type
(`in_place_forms`), the net enumerates those pairs from it, and consumers
call the form's step instead of re-deciding from types.

## Why

- **Correct by construction.** A form that answers its own type's row is
  right whichever consumer reaches it, generic code included, which no
  consumer-side rule can be.
- **Readable where it's free.** The type's answer leaves every slice,
  `Vec`, range and std source's `filter_map` as JS writes it. Only crate
  iterators with their own `size_hint` and generic code's sources move:
  in the corpus, two cases (generic functions); in React's port, none.

## Alternatives

- **The form's answer** (does the made Expr carry the crate's
  `size_hint`?), with the type's. Tried: over the whole corpus it never
  changed a line the type's answer didn't, and it reads JS shapes.
- **Always `$somes`.** Correct, but every `filter_map` of a slice would
  lose upstream's `filter((item) => item != null)`.
- **Not in place there** (std's compiled struct). Correct, but the
  struct's source form would have to follow the same choice.

## Consequences

- Corpus `flatten_of_options_counted_asks_no_hint` holds it (counts where
  they're made, and in generic code); `filter_ends` and
  `filter_map_of_given_from_its_back` moved to `$somes`; React's port is
  unchanged.
- knowing.rs makes `$somes` of a source stepped from its back of that
  source made knowing it, as it made `$filterEnds`.
- Mutations: `somes-counted-as-a-filter`, `somes-of-a-back-forward-only`,
  `somes-never`, `somes-of-any-type`, `somes-not-made-knowing`.
