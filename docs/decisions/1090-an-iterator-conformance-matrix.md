# 1090. An iterator conformance matrix, against native Rust in both builds

Status: Accepted, the user's decision of 2026-11-04: "a generated iterator
conformance matrix, compared against native Rust in both builds". Extends
[0182](0182-boundary-matrices.md), [0908](0908-one-meaning-per-iterator.md)
and [1030](1030-what-only-the-nightly-runs.md).

Case ([0262](0262-when-rust-and-js-disagree.md)): none for programs; this
is a test, and when it runs.

## Context

A study of the 83 wrong answers (`P0: wrong answer`) filed from
2026-10-29 to 2026-11-04 found 21 of them were one protocol: what each of
`Iterator`'s steps does, which std's adapters specialize (`Skip::fold` is
`nth` then its source's `fold`; `Take::nth` past its end drops what it
takes), what they drop, and what they drop as a panic unwinds. rust-js
reproduces it in three hand-kept places, the runtime's adapters, the step
lists `$given` and `$iterator` read, and one recognition predicate a step,
and a second time in std's compiled structs (`--std-iterators`, ADR 0908).
Each new combination of adapter, step and source needs editing several of
them, and one missed is a wrong answer.

Hand-written probes and the ports found 16 of the 21; the generated
programs (ADR 0092) and surviving mutations almost none. The probes behind
#491–#496, #505 and #615 were each a slice of one matrix: a source, an
adapter or two, a consumer and where a panic is, compared with native Rust.

## Decision

**`test/iterator-matrix.ts` generates that matrix; `test/iterator-matrix.test.ts`
runs each case natively and as JS in both builds, the recognizers' and
std's structs alone, and compares each case's lines.**

- **Its dimensions:**
  - **source:** a `Vec`'s `into_iter()`, reversed, its `iter()`, an array's
    `iter()`, a range, and the crate's iterators, one owning each of
    `fold`, `nth`, `last`, `count`, `size_hint`, none, an exact-sized one,
    a double-ended one and one owning every step; and a type parameter `I`,
    a `Vec`'s or the crate's, given to a generic `fn`;
  - **chain:** none, one or two of `map`, `filter`, `skip`, `take`, `fuse`,
    `peekable` (peeked first), `enumerate`, `chain`, `rev`, `copied`,
    `cloned`, `step_by`, `flatten` and `zip`;
  - **consumer:** `fold`, `rfold`, `try_fold` (broken off), `any`, `all`,
    `count`, `last`, `nth` and a `next` after it, `sum`, a `for` (broken
    off), and `collect` into a `Vec`, a `String`, a `HashSet` and a
    `HashMap`;
  - **panic site:** none, the crate's `next`, the chain's first closure,
    the consumer's closure, or an item's `Drop`, each run under
    `catch_unwind`.
- **What it observes:** every item, accumulator, `sum`'s total and the
  crate's iterator print as they're dropped, the crate's own steps print as
  they run, `clone` prints, and each case says whether it panicked and
  with what. What a hash collection holds is a number, which no drop
  prints, as its order is its hasher's natively.
- **Impossible combinations are never made,** by construction: each
  adapter says what it takes and makes (an item, owned or borrowed, a
  pair; double-ended; exact-sized), each consumer what it takes, and a
  panic site is only where it can be. No list skips a case. It makes
  101,973 cases.
- **Deterministic and seedable:** the cases are made in one order and
  run in batches of 60, one program each, the same batches in every run.
  A run of all of them (`ITERATOR_MATRIX=all`) is split into parts
  (`RUST_JS_CASES=i/n`); a sample is `ITERATOR_MATRIX_BATCHES` whole
  batches, `ITERATOR_MATRIX_SEED` saying which, so a case is judged in
  the same program whichever runs it.
- **A refusal is rust-js's answer:** a case rust-js refuses is left out of
  its batch's JS, found by where the error points, and the batch compiled
  again. A run that ends early ended in the last case it began; the cases
  after it run again without it. A crash of rust-js is a failure.
- **Known failures only shrink:** `test/iterator-matrix-known.txt` lists
  each case that doesn't run as Rust does yet under its issue, by build, as
  ranges of its number in the generator's order (`plain 120-179,4410`),
  with the generator's version, a hash of every case's name: a change to
  the cases is a new version, and the list is made again with each
  issue's cases found again. A case listed that runs as Rust does, or is
  refused, fails until it's taken off (`bun test/iterator-matrix.ts
  take-off`, from what the runs wrote); one not listed that differs fails,
  its program alone written to `target/iterator-matrix/`
  (`bun test/iterator-matrix.ts program <n>` prints it too).
  `ITERATOR_MATRIX=#<issue>` runs the batches that hold an issue's cases,
  for its fixer.
- **When it runs,** as the quality/velocity rule asks (ADR 1030):
  - every case each night, nightly.yml's `matrix`, on six machines of four
    parts each, and `bun run nightly matrix` here;
  - a sample of the day's 8 batches, of a push to `main` that changes
    `src/runtime/`, `src/lower/iterators/` or `src/lower/mir/iter/`, by
    iterator-matrix.yml, beside its check, not in it;
  - not in a push's suite: the file is in `scripts/shard.ts`'s `nightly`.

## Why

- **It's the probes, generated.** Each P0 the probes found was a cell of
  this matrix; every cell is now run every night, in both builds.
- **Both builds against native Rust,** where ADR 0908's forms test
  compares std's structs on the corpus's cases alone.
- **A panic and a drop are the observable:** 14 of the 83 were seen only
  as something panicked, 20 only in what dropped and when.

## Alternatives

- **Corpus cases, as the boundary matrices are:** 101,973 cases don't fit
  as committed files, and each would be compiled alone.
- **A covering array,** each pair of values once: smaller, but the bugs
  are of three or four dimensions at once (#625: `enumerate().map()`,
  the crate's own `fold`, `fold`).
- **Every case each push:** about 4 CPU-hours.

## Consequences

- At the commit that added it, 32,186 of its 203,946 case-builds (16%)
  don't run as native Rust does, and 4,921 are refused. Each failing case
  is listed under one of 21 issues: m6-wife's #621–#626, #637 and #640,
  and #644–#656, filed from it, one a mechanism, found by what differs
  (a drop more or fewer, an order, an own step not called) and where
  (the chain, the consumer, the panic site). The list's attribution is by
  that rule; a fix may take off cases listed under a neighbour.
- After m6-wife's fixes of #621–#627, #637 and #640 (22328e474), a run of
  every case took 15,294 off and found 31 more, 13 a `try_fold`'s
  accumulator now dropped where rustc leaks it (#651): 16,923 are listed,
  #622 and #626 none.
- The simplest cells fail: a crate iterator with a `Drop` over items with
  one, folded with no adapter and no panic, drops twice (#637); a `Vec`'s
  `rfold` whose closure panics leaks what's left (#650).
- A run of every case took 2,970 s on the M3 Max in 4 parts at a load of
  15 to 60 (about 3.3 CPU-hours): the nightly's, on six machines of four
  parts, about a quarter of an hour each after the compiler's build. A
  push's sample of 8 batches takes about a minute after the build.
