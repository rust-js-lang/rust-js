# 1098. The compiler's maps are walked in the program's order

Status: Accepted (2026-11-07, #866). Extends ADR 1003: a compiler's
build is its identity only if the same build gives the same output of
the same input.

Case ([0262](0262-when-rust-and-js-disagree.md)): none. This changes
what decides the order the compiler walks its own maps in, not what Rust
or JS does. Every corpus snapshot, both builds and std's JS are
byte-identical across the switch.

## Context

The same program compiled eight times by one compiler gave two JS forms
of a `u32` function's `n + 1` (#866): `return n + 1;` 3 times, `return
(n + 1) >>> 0;` 5 times. Ranges' fixpoint (ADR 0577) walked the callees
it had told otherwise in a `HashSet<DefId>`'s order, and forgot what
kept changing after four changes. Whether a callee reached its fifth
change depended on that order, and a std `HashSet`'s order comes from
random keys its process picks. Both forms were right; the output
still differed from one compile to the next. The corpus's snapshots, the
verified runs' keys and every library's manifest assume it doesn't:
the corpus's library check failed on one run and passed on the next.

rustc's own lint for this, `rustc::potential_query_instability`, found
519 walks of a std `HashMap` or `HashSet` in `src/`. A walk is order-free
where what it makes is too (another set, an `any`, a count), and
order-dependent where what's decided, or written, follows the order: a
fixpoint that widens after so many changes, a first-come name, a list
written out. The whole corpus, compiled twice in separate processes,
gave no other difference, so few of them reach the output today. But
each is one edit away from it, and nothing would tell.

A fixed hasher would make each walk the same from one compile to the
next only where the keys hash the same: a `Ty`, a `GenericArgsRef` or a
pointer hashes by its address, which ASLR moves in each process.

## Decision

**The compiler's maps and sets are `crate::collections`' `IndexMap` and
`IndexSet`**: indexmap's, with rustc's Fx hasher. Each is walked in the
order its entries were put in, which the program alone decides, so every
walk is the same in each compile, whatever its keys and whatever it
decides.

- **std's are refused**: test/architecture-maps.test.ts fails on a
  `std::collections` `HashMap` or `HashSet`, a `hash_map::`/`hash_set::`
  path or a `RandomState` in `src/`, naming the file.
- **An entry is taken out by `swap_remove`**, which puts the last in its
  place: an order the program decides too. Nothing read the order of
  what a std map had left.
- **A net**: test/determinism.test.ts compiles a corpus case again, as a
  program and as a library, and compares its JS, source map and
  manifest byte for byte. The nightly's suite compiles the cases that
  were two forms eight times each, and the nightly's `determinism` job
  compiles the whole corpus twice (`DETERMINISM=corpus`, in parts).
  It's the net for any other source of difference, a time, a path or a
  thread, which the architecture test can't see.

## Why

- **By construction, not by audit**: an audit of 519 walks finds the
  order-dependent ones of today, and the next walk added is unaudited.
  An insertion-ordered map makes every walk deterministic, and the
  architecture test keeps it so. Determinism is a property of the
  collection, as rustc's own `FxIndexMap` is used where its output's
  order matters.
- **One way**: one map type for the compiler, not std's where order
  happens not to matter and another where it does. Which walks are
  order-free is not a question anyone has to answer again.
- **No cost**: the corpus compiled twice took 64 s before and after on
  the M6 mini (12 cores), as rustc's front end is most of each compile.
  The Fx hasher is faster than std's SipHash where the maps are hot.
- **The net by its cost**: the corpus's own snapshots compare each
  push's one compile, which caught #866 at random. The determinism job
  is about 25 CPU-minutes, so it's the nightly's (ADR 1030).

## Consequences

- `IndexMap::new()` is `IndexMap::default()`, `HashSet::from([..])`
  `IndexSet::from_iter([..])`, `drain()` `drain(..)`, and a lookup
  takes the key's type, `contains(*id)`, not a reference to a
  reference. Mutations whose `find` or `replace` named std's maps name
  `collections`' now.
- A `swap_remove` changes the order of what's left, which is
  deterministic; code that needs the order of insertion after a removal
  would use `shift_remove`, and none does today.
- std's `crate_hash` in `std/manifest.json` still differs between
  sysroots built in different worktrees, as before: std-package.test.ts
  leaves it out of what it compares. That is the sysroot's build, not a
  map's order, and isn't changed here.
- rustc's query-instability lint found the walks. It needs
  `-Zunstable-options`, which flags rustc's internal-API uses here too,
  so it isn't part of clippy's run; the architecture test is.
