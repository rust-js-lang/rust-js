# 1183. The MIR intervals' states share what they hold

Status: Accepted (2026-11-22, #2310; the lead chose the dependency over
hand-written persistent maps; #2308 folded in).

Case ([0262](0262-when-rust-and-js-disagree.md)): none. This changes how
the compiler holds what the MIR intervals know, not anything they prove or
the JS does.

## Context

The MIR intervals ([ADR
1137](1137-the-bounds-analysis-on-mir-shadow-first.md)) walk a body to a
fixed point. They keep a `State` at the entry of every block, and narrowing
keeps another for every block's flow out. A block's state is a copy of what
flows into it with a few facts changed. Each is held whole, so a state is as
large as the facts known so far, and nothing drops what's known of a local
once it dies.

A function of `n` calls has about `n` blocks, and each call leaves facts on
its temporaries: an interval, a copy and a sum for `v.push(n + i)`. Its
memory was quadratic. test/lowering-functions.test.ts's function of 3,000
statements took 5.35 GB, 1,500 took 1.47 GB, and 750 took 0.46 GB. A long
generated or hand-written function could exhaust memory. `State.intervals`
was also dense: a slot for every local, 48 bytes each, in every block's
state.

The same structure cost time ([#2308](https://github.com/rust-js-lang/rust-js/issues/2308)):
the walk cloned a `State` per edge, about 6% of the corpus's compiler CPU
and 21% of the React port's.

## Decision

**A `State`'s maps and sets are persistent: a clone shares everything, and a
change copies only the path to what it changes.** `analysis/intervals/shared.rs`
holds `SharedMap` and `SharedSet`, built on the `imbl` crate's `OrdMap` (pure
Rust, so the wasm playground builds it too). `State.intervals` is a
`LocalIntervals`, an `OrdMap` of the locals that have an interval. It reads
as the slots did (`intervals[x]`, none where it has none), walks in the
locals' order, and is shared in the same way.

**`shared.rs` is the one owner of what these collections mean, and that is
an `IndexMap`'s and an `IndexSet`'s.** Every answer the walk gives stays the
same by construction:

- Each collection walks in the order its keys were first inserted.
- A key inserted again keeps its place.
- A removed key leaves the others in their order (`shift_remove`).
- `union` and `intersection` keep `IndexSet`'s orders.
- Two collections are equal when they hold the same entries, in any order.
  Equality short-circuits on pointer equality: a block's state unchanged
  from what flows into it is equal without being read.

The unit tests in `shared.rs` mirror 20,000 random operations on an
`IndexMap` and an `IndexSet`. Each answer, order and equality is compared,
and every clone taken along the way must keep what it was given.

## Alternatives rejected

**Forgetting a local's facts at its `StorageDead`.** It is sound, since
forgetting only weakens what's known, and it made memory linear (0.23 GB at
3,000 statements). But it costs proofs. A dead local's facts are still
reachable through relations, and through lookups by value: `sums`' "`x + c`
again is `t`" gives a dead `t`'s interval and relations to the next
`x + c`. Across the corpus, `byte_escape`'s `bytes.slice(0, len)` became
`$slice(bytes, 0, len)`, among others. Any rule that drops facts loses
precision somewhere.

**A hand-written persistent map.** About 300 lines of our own to review and
keep correct, where a maintained crate does the same.

## Consequences

- Memory is about linear in a function's length. The 3,000-statement
  function takes 0.29 GB and the 1,500-statement one 0.20 GB.
  test/lowering-functions.test.ts holds this: twice as long must take
  well under twice the memory (quadratic was 3.6 times), and under 1 GB.
- The corpus's and the React port's JS are byte-identical.
- The compiler depends on `imbl`, pinned in Cargo.lock.
- Time is still quadratic in a long function's length: `forget` scans each
  map at every write, and a join compares states. #2310 records this; it is
  a constant-factor cost here, 8 s for 3,000 statements in a debug build.
- A new `State` field takes a `SharedMap` or a `SharedSet`. An `IndexMap`
  field would be held whole in every block's state again.

## Amended: the walks for what callers give share their fixed points (#2313)

`parameter_intervals` and `capture_intervals` walked a body again with the
same `Given`: a recursive component's settled round, and a closure's
maker after its callers' walk, 41% of what they walked in the React port.
`FixedPoints` (`intervals/fixed_points.rs`) lets them share what they
found. Those two walks alone share it: neither knows the crate's
libraries or the effects query, and both run once `closure_results` has
set the closure results the transfer reads, so a body and its `Given`
decide everything a walk knows. `closure_results`' own walks, before
those results exist, and the proofs' walks, which know the libraries,
share nothing. Only what's asked again is kept: a recursive component's
members' fixed points while its rounds run, and of each closure's maker
what's known at each closure it makes. Keeping every body's fixed point
cost 17% more memory in a crate of the port.

Since #2310 shares the states, walking every body costs time, not
gigabytes: serde's example takes 339 MB with it, 273 MB without, so the
test's 2 GB bound no longer catches it. `--intervals-shadow` now says how
many bodies the walks walked, `walked-for-callers<TAB>walked<TAB>bodies`,
and the test asserts that count.
