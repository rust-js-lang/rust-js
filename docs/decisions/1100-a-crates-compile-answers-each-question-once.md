# 1100. A crate's compile answers each question once

Status: Accepted (2026-11-07, #884). Amends
[0914](0914-a-librarys-generic-function-is-its-programs.md)'s walk of
what a library's function copies; extends
[1098](1098-the-compilers-maps-are-walked-in-the-programs-order.md)'s
`collections` with `OnceMap`. Amended (2026-11-07, #896): every build of
the compiler is optimized, `opt-level = 1` without LTO.

Case ([0262](0262-when-rust-and-js-disagree.md)): none. How long a
compile takes, not what it writes: React's port regenerates byte for byte
as its pin committed it, and every corpus snapshot is unchanged.

## Context

React's port regenerated in 8 to 9 minutes on the 12-core M6 (`bun
scripts/react-in-rust.ts regen`, #884), its cores a fifth busy. Its
workspace is one `cargo check` (ADR 1068), so Cargo runs the crates that
don't use each other at once; but the crates are a chain, `shared`,
`react-isomorphic`, `react-reconciler`, `react-server`,
`react-dom-bindings`, `react-dom`, and three crates that use the server,
`react-markup` and two Flight servers', took as long as the reconciler.
Each crate is one `rust-js` on one core: the time was its compile's.

macOS's `sample` of each crate's `rust-js`, through a regen, said where:

- **`std_path` was 40–58% of every crate's compile.** Every question of
  whether a type is std's `Map` or `Rev` printed the item's path with
  `def_path_str`, which walks std's modules for the visible path each
  time; `is_compiled_std_type` printed it once for each of `COMPILED_STD`'s
  entries. What a type drops asks it of each of its parts.
- **What a library's function copies was walked again for each way to
  it** (`instances::Copying`, ADR 0914). An answer that read a function
  still being asked, one of functions that call each other, wasn't kept,
  so it was found again from each caller: exponential in React's Flight
  servers' mutual recursion. `react-markup`, `react-server-dom-esm` and
  `-unbundled` spent 140–340 s each there.
- **What dropping a type runs was found again for each body**
  (`drop_runs`, with a `TypeDrops` of its own each call): the crate's
  analysis asks it of the same types in each of its bodies, after the
  facts it depends on, the `Rc`s counted and the closures' facts, have
  settled. 30% of `react-server`'s compile.
- **`compiled_provided_method`** found `Iterator::rev`'s `DefId` by
  walking `Iterator`'s items, for each of `HELD_COMPILED`'s makers, at
  each call asked whether it makes one: 10% of the reconciler's.

## Decision

**A question whose answer can't change during a compile is answered once
and kept for the rest of it**:

- **`std_path`** keeps each std item's path, a session's `OnceMap`
  (`recognition/std_crates.rs`, where std's crates moved, the question of
  which crate is std's and the path of its items). `has_compiled_bodies`
  finds the types of `compiled_std`'s impls once, as `compiled_std` is
  found; `compiled_provided_method` keeps each trait's method found.
- **`Copying` is Tarjan's walk.** Functions that call each other copy
  where any of them does, so each has the answer of the first of them
  asked; one asked while it's still being asked answers what that first
  will, and all of them are kept when the first is answered. Each function
  is walked once. The answer is the least that holds, as before, so the
  same.
- **`DropRuns`** (`drops.rs`) keeps `drop_runs`' answer of each type, in
  each typing environment, from where the analysis's facts settle,
  `closure_facts.cloned_apart` set, to its end; the borrow checker holds
  the facts unchanged while it lives. Before then, each call is its own.
- **`collections::OnceMap`** is a session's memo: each key keeps the
  first value it's given, never changed or taken out. Recognition holds
  no `RefCell` of its own (test/architecture.test.ts); a `OnceMap` answers
  what recognition would find, so it reads no state.

Each memo is a thread-local, as `COMPILED` was, or lives in the
analysis: a compile is one thread's (`on_its_thread`, and WASI's one
process), so none outlives the compile, and none is kept on disk. Each
holds one entry for each item or type the crate asks of, so it's bounded
by the program, not a cache with a cap: nothing goes stale while it
lives.

## Why

Measured on the 12-core M6, the regen after a change to the compiler,
each crate compiled again, its JS the pin's byte for byte at each step:

| Step | Regen | CPU | `react-reconciler` |
|---|---|---|---|
| main (f881cd1d8) | 504 s | 1,309 s | 123 s |
| `std_path` once an item | 236 s | 721 s | 32 s |
| `Copying` Tarjan's | 92 s | 128 s | 32 s |
| `compiled_provided_method` kept | 84 s | 118 s | 30 s |
| `DropRuns` | 75 s | 105 s | 23 s |

The regen as landed: 71 s after the change from main, 75 s after a
one-line change to `src/` (its build 2.7 s), 79 s from a fresh fetch of
the port, against main's 504 s at 260% CPU; 105 CPU-seconds, not 1,309.
With no change, Cargo has every crate as done (ADR 1068): 1.9 s. A cache
of each crate's JS by its inputs (#874's) saves nothing more: a change to
the compiler changes every crate's key, as its build is its identity
(ADR 1003), and with none Cargo's fingerprints are already that cache.

## Alternatives

- **Cache each crate's JS by its exact inputs, sharing #874's.** Pays
  only where the compiler didn't change, where Cargo already skips
  every crate; the regen runs after a compiler change. Not built.
- **A compiler built with `opt-level = 1` for the regen.** The regen in
  31 s, not 75 s, as each `rust-js` runs three times as fast; but a
  build after a one-line change takes 15 s, not 2.7 s, and a second
  build's target, 1.6 GB. Worth deciding for the whole loop, the tests'
  compiles too, not for the regen alone (#896): decided so, below.
- **Compile each crate's bodies on several threads.** rustc's `TyCtxt` is
  one thread's here; the work was repeated, not parallel.
- **Each memo a `RefCell` in recognition.** Recognition holds no mutable
  state (test/architecture.test.ts); a `OnceMap` says what it does.

## Consequences

- A compile of React's reconciler, alone, takes a quarter of the time; the
  corpus's and the tests' compiles gain where they asked the same.
- The regen's critical path is now `react-reconciler` (23 s),
  `react-server` (14 s) and `react-dom-bindings` (22 s), one after the
  other. Of `react-dom-bindings`, a third is tidy's `ending_returns`,
  which walks an `else if` chain's branches twice at each level, two to
  the depth (#895).
- `std_items.rs`' questions of std's crates are `std_crates.rs`', and its
  mutations `scripts/mutations/lower/recognition/std_crates.ts`.

## Amended: every build of the compiler optimized, without LTO (#896)

**Cargo.toml's `[profile.dev]` is `opt-level = 1`, `lto = "off"`**, one
profile for every build of the compiler the loop runs: a build after an
edit, the tests' and `check:fast`'s, `cargo test`'s, the mutations'
(ADR 1031), a port's regen, and CI's, which builds from the same
Cargo.toml. Not a second profile for one step: a second compiler is a
second build and a second target, and a step that measures one compiler
and ships with another.

Case ([0262](0262-when-rust-and-js-disagree.md)): none. How fast the
compiler runs, not what it writes: debug assertions and overflow checks
stay on, as in any dev build, and the corpus, the suite and React's regen
are unchanged.

Measured on the 12-core M6, alone, after a one-line change to `src/`
(docs/dev-loop.md has every profile tried):

| | `opt-level = 0`, before | 1, LTO off |
|---|---|---|
| The build, a comment's or a code change | 2.8 s | 2.5–2.6 s |
| `check:fast --all` | 167–175 s | 127–140 s |
| React's regen | 75 s | 28 s |
| The corpus, 12 parts | 52 s | 29 s |
| A mutation run, `target/mutants` from nothing | 23–31 s | 48 s |
| Its mutant's build | 4.3 s | 2.6 s |
| `target/debug` after a check | 2.5 GB | 2.3 GB |

At `opt-level = 1` rustc's default is LTO of each crate's own code units
("thin local"), which re-optimizes across them after any change: a
comment's change built in 7.2 s and a code change in 12.8–15.3 s, as
#896 first measured. Without it, a change compiles only its code units,
as unoptimized, and the compiler runs as fast. At 2 it runs no faster,
and builds from nothing in 47 s, not 29 s; with only the dependencies
optimized the gain is a tenth, as the time is rust-js's own code.

What it costs: a build from nothing, sccache warm for the dependencies,
29 s, not 17 s, which each mutation run pays where its `target/mutants`
was removed; and CI's first build after the change, which compiles the
dependencies again once. The release builds, rustc's tests' (ADR 0089)
and the packages', are another profile, for reasons of their own.

## What holds it

- React's port, `bun scripts/react-in-rust.ts regen`: its JS byte for
  byte as the pin committed it.
- `test/crates.test.ts`' "a library's generic function is its
  program's", of `outer` and `inner`, which call each other above one that
  copies.
- Mutations `copies-settled-alone` (`instances.rs`), and
  `drop-runs-settled-asked-crate-wide` (`drops.rs`); `std-path-of-any-crate`,
  `std-path-through-alias`, `no-std-paths-core` and `std-crate-by-name`
  move with `std_path` to `std_crates.rs`.
