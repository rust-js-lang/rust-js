# 1103. The corpus shuffled into crates finds what one function tells another

Status: Accepted (#853; the net the user's queue asked for, 2026-11-08).

Case ([0262](0262-when-rust-and-js-disagree.md)): none decided here. This
is a net: each wrong answer it finds is its own issue, decided there.

## Context

#823, #834 and #850 were wrong only beside another function in the same
crate, one that made a fact about a type true, "this type is stepped from
its back" say, which the first function's JS then read. The iterator
matrix's full run missed #850, because its two rows were never in the same
batch; #818's rebatching found it by chance. Such a fact isn't only the
iterators': drops, nullability's `nulled_params`, dictionaries and any
lookup by a name rather than by a `DefId` are crate-wide too. A corpus
case, compiled alone, can't show one.

## Decision

**The corpus's cases, shuffled into groups, each group one crate**
(`test/neighbours.ts`, `test/neighbours.test.ts`):

- A seed shuffles the cases that can share a crate and cuts them into
  groups of 5 to 20 (`groups`), so a run is reproduced from its seed. A
  case can't share one where its text resolves at the crate's root or
  defines what two cases would both define there, `crate::`, `super::`,
  `$crate`, an inner attribute, an exported macro, `extern crate`, a module
  in another file or a symbol's own name; or where it panics out of
  `main` or aborts, or is of another edition or of `wasm32` (`apart`).
  1,652 of the corpus's 1,846 are in.
- Each group's crate holds each case in a module of its own, `mod n0 {
  include!("case.rs"); }`, whose `main` a driver calls in turn after a
  marker line (`crateOf`). It runs natively and as JS in both builds,
  through the corpus's own `runNative`, `compileJs` and `runJs`; each
  build compiles in a folder of its own, as each module is a file of its
  own, `n0.js`, which another build would write over.
- A group that runs otherwise is **narrowed**: the marker lines find the
  first case whose part of the output differs, the victim, and the
  neighbours are halved, then dropped one by one, while the victim still
  differs (`narrowed`). A group refused or crashing together is narrowed
  the same way to the case its error points at, by the error's first line
  with each module's name, `DefId` and number made alike (`failure`), or,
  of a crash that points at none, to the fewest of the group that still
  crash (`fewest`). What's filed is a pair.
- **test/neighbours-known.txt** lists each pair under its issue, its builds,
  its victim and its neighbours, `slow` of one that runs as Rust does but
  too long. A group whose only problems are listed pairs passes, and each
  listed pair is compiled alone on each run, so a fixed one fails until
  it's taken off: the list only shrinks.
- **Where it runs.** Each push runs the listed pairs and seed 0's first two
  groups, about 25 s. The nightly's `neighbours` job runs every group of
  the day's seed, the generated programs' day number, in four parts, so
  each night is a new shuffle (`NEIGHBOURS=all`, `NEIGHBOURS_SEED`).

## Alternatives

- **Pairs of every case with every other**: 1.4 million crates. Groups of
  5 to 20 put each case beside 4 to 19 others a night, and a different
  set each night.
- **The cases concatenated, not in modules**: their items' names collide.
  A module of each keeps them apart as two modules of one real crate are,
  which is the shape #951 and #952 crash on.
- **Narrowing by hand**: a group's output differing is a group of 20 to
  read. The markers and the halving give the pair in about ten compiles.

## Consequences

Its first run, seeds 0 to 4, about 640 groups, found eight issues that no
corpus case, table or full run had: two wrong answers, #949 (a `TypeError`
of a guard's deref beside an `Rc<fn>`'s `ptr_eq`) and #950 (a per-type
`rev()` fact that makes a range's skip step 4e9 items where alone it moves
the end, 15 s where native's is instant); two crashes, #951 and #952, each
beside a case that defines a type of the same name in its own module; and
four refusals of a pair whose cases each compile alone, #953 to #956, one
of them `{:?}` read as `{:#?}`. It found one fault of its own first: the
two builds compiling into one folder ran one build's case beside the
other's modules. `docs/finding-wrong-answers.md` lists it among the
techniques; TESTS.md lists the test file.
