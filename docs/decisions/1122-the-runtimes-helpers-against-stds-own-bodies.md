# 1122. The runtime's helpers against std's own bodies

Status: Accepted. Extends [1090](1090-an-iterator-conformance-matrix.md)
and [0908](0908-one-meaning-per-iterator.md).

Case: none of [0262](0262-when-rust-and-js-disagree.md)'s: a net, no
change to what any program does.

## Context

rust-js's runtime copies much of std by hand: its iterator adapters and
consumers, `extend`, `splice`, capacity growth, size hints, the `Debug`
builders (#1205). Each helper is a second copy of std, which can drift
from the first. rust-js can also compile std's own bodies: with
`--std-iterators` (ADR 0908) std's iterator structs are its JS, in place
of the recognizers' helpers. The tables (ADR 1090) run each case in both
builds against native Rust, but native's compile costs as much as both
builds' together, so a push runs a slice of them.

## Decision

**A net pits the plain build, the runtime's helpers, against the build of
std's own bodies, case by case, with no native run: `ITERATOR_ORACLE=std`
of `test/iterator-matrix.test.ts`.** The std build's printed lines are the
oracle. A case it refuses, or whose run ended in it, has none and isn't
judged; a case either build's known list names isn't either.

- **Per push of the iterators' code** (`twinPaths`, the runtime's among
  it): `bun scripts/iteratorSlice.ts run twins`, check.yml's `twin-net`
  and `check:fast`'s. It runs all of the conformance, points, unwind,
  consumer and std tables, and the matrix's `pairs` sample. That's 53,082
  cases, 49,554 of them judged, in 56 s on the 12-core Mac. A batch costs
  about half of a run against native.
- **What it pits**: a helper the plain build of a batch imports and its
  std build doesn't, as std's body ran in its place. That's 35 helpers:
  the lazy adapters, `$rev`, `$stepBy`, `$flatMap`, `$lazyZip`, `$sizeHint`,
  `$foldBack` and their kin.
- **What it can't, `shared`** (`test/twin-net.ts`): each other helper the
  plain build imports, which the std build imports too, with why std's
  build has no body of its own there. Of 231: glue, how JS holds a value or
  a panic (47); `Iterator`'s provided consumers, which std's build lowers
  as the plain one does (33); iterators whose structs it doesn't compile,
  a slice's, `Chars`, `Repeat` (12); ranges, `Step`'s (8); collections'
  methods, `extend`, `splice`, capacity, maps (122); sorts (4); `Debug`
  (5). After a whole run, the runner fails on a helper neither pitted nor
  listed, and on one listed that's pitted now or that nothing imports.
  `test/twin-net.test.ts` holds each listed name to a helper the runtime
  declares.
- **The nightly doesn't run it**: its tables run every case in both builds
  against native Rust, which holds the two to each other too.

## Why

- **It's cheap**: no native compile, so a push of the runtime's or the
  iterators' code runs every table but the matrix whole, in parallel parts.
- **It names the copies**: `shared` is the list of the runtime's copies
  of std that nothing yet pits against std's own, the next net's work.

## Consequences

- Its first run, of all of the matrix too (131,205 cases, 129,843
  judged), found none that differ.
- Its `shared` list is longest for the collections' methods, which no
  build compiles from std's bodies yet: pitting them needs a build of
  std's `Vec`, `String` and maps, which `--std-iterators` isn't.
- TESTS.md has its row; docs/finding-wrong-answers.md its technique.

## Amendment: a push runs each table's slice (2026-11-11)

Case: none of [0262](0262-when-rust-and-js-disagree.md)'s: no program
changes.

The whole net took 9 min 15 s as CI's `twin-net` job (run 42887915279),
more than a push's check should wait for. A push runs each table's slice
now, the sample its run against native takes, 63 s on the Mac in 4
parts. The night runs all of it, nightly.yml's `twins`, `bun
scripts/iteratorSlice.ts run twinsAll`. A slice may see a helper only
where both builds import it, where a whole run pits it, so
`test/twin-net.ts` lists the pitted ones too, `pitted`. A run fails on a
helper both builds import that neither list names. A whole run also
fails on a list that says what it doesn't: a `shared` helper pitted now
or imported by none, a `pitted` one no case pits, one pitted that
`pitted` doesn't list.

## Amendment (2026-11-13, #1458): each unpitted iterator says why, by its own kind

`shared`'s `iterators` kind gave one reason to helpers of three different
cases. It is now three kinds, so a reason never covers both what's decided
and what's merely not done yet:

- `js-own`, a std iterator stepped as JS's own iteration by decision,
  each with its ADR beside it: `$iter` (ADR 0071), `$intoIter` (ADRs 0061,
  0475), `$advanceSlice` (ADRs 0422, 0465), `$chars` (ADR 1117), `$next`
  (ADR 0140) and `$nextSome` (ADRs 0140, 0051);
- `memory`, a std method whose body reaches memory, which std's build
  can't compile: `$peek`, `Peekable::peek` (ADR 0561);
- `to-pit`, a plain std struct std's build doesn't compile yet, each to be
  pitted in turn, its std body compiled in the std build: `$repeat`,
  `$repeating`, `$repeatingN` (`Repeat`, `RepeatN`), `$cycleOf` (`Cycle`)
  and `$decodeUtf16` (`DecodeUtf16`).

`iterators` keeps a slice's lazy searches. The ranges' eight stay `ranges`
until a probe of compiling `Step`'s bodies says which they are.

The probe (2026-11-13) keeps the ranges' helpers, seven, `ranges` (#1589):
compiling `Range`'s, `RangeInclusive`'s and `RangeFrom`'s `Iterator` and
`DoubleEndedIterator` bodies, std's build meets `expect`'s literal panics
(`core::option::expect_failed`, as #1569's `Repeat`), the specialized
traits `RangeIteratorImpl` and `RangeInclusiveIteratorImpl`, beside the
iterators in `core::iter::range` where the types are `core::ops`', a
comparison of a type parameter by its `PartialOrd` dictionary, each
integer's `Step`, and, the largest, a range as std's struct wherever the
build makes one, where every lowering of a range is the recognizers'.

`$repeatingN` is pitted now, std's `RepeatN` compiled in the build of
std's bodies (ADR 0932, amended). `$repeating` is `memory` (#1569):
`Repeat`'s `last` and `count` inline a pointer transmute std's build
can't lower. `$repeat` was listed `to-pit` by
mistake: it's `vec![item; count]`, `Vec::from_elem`, a collection's, not
`Repeat`'s, so it's `collections`.

`$cycleOf` is pitted now, std's `Cycle` compiled in the build of std's
bodies (ADR 0932, amended).

`$decodeUtf16` stays shared, listed `iterators`, and `to-pit` is gone
with it (#1584): `DecodeUtf16Error` is its code (ADR 0327), where std's
body makes `DecodeUtf16Error { code }`, so pitting it needs std's struct
table to reach past `std::iter`, a `char` module in `@rust-js/std`, that
literal written as its one field, and `u16::is_utf16_surrogate` lowered,
four mechanisms for one helper the corpus already holds against native
Rust (the lead's decision for the user, (b)).
