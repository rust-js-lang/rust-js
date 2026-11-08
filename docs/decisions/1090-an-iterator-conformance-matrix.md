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

## Amendment: a per-push slice, differential, with captures and pairs (#731, 2026-11-05)

The user's decision. A push's sample of 8 batches was the day's (a seed),
in a workflow of its own beside the check. It missed #721 (a regression
of #702 found in the nightly's batch 258) and #722, and the matrix's
closures captured nothing, which is how #710, #719 and #734 got through.

- **Two variants, appended after the plain cases**, so the plain cases'
  numbers, and test/iterator-matrix-known.txt's, are as before (its
  generator line moved to the cases' new version, `13cf57ce0f18`). Of
  chains of none or one adapter: `captures`, each closure of the chain
  and the consumer capturing a `D` of its own by move (3,999 cases); and
  `paired`, the type first stepped from its back by `rfold`, `nth_back`
  or `rfind` beside the forward consumers `count`, `last`, `nth`, `vec`
  and `fold` of the same type, written again where its chain has no
  closure and made by one closure where it has (1,200 cases). A pair puts
  a crate-wide fact, which #702's knowing decides by every use in the
  crate, next to what reads it. 107,172 cases in 1,787 programs.
- **The slice is fixed**, `slice()` in test/iterator-matrix.ts: the fewest
  whole batches, picked greedily, the first of equals, that hold every
  stratum any case has: of a plain case, its source, each adapter and its
  panic site, each with its consumer, and its source and consumer with its
  panic site; of a capturing one, each adapter with its consumer, and its
  source and consumer with its panic site; of a paired one, its setter
  with its source and chain, and with its consumer. 64 batches: 22 plain,
  22 capturing, 20 paired. A failure is found again by running it again.
- **Both builds against native Rust, as before:** a build that differs
  from native fails unless that build's case is listed, so a difference
  between the builds where native agrees with one fails the other.
- **It runs in each push's check**, check.yml's `iterators` job, where
  `plan` says the change since the last good check is the iterators'
  code, and in `check:fast` where the change is. What that code is,
  `iteratorPaths` in scripts/iteratorSlice.ts, is said there alone;
  test/architecture-iterators.test.ts holds it to docs/architecture.md's
  owners of iterators and their steps. iterator-matrix.yml is removed.
- **What it found at once:** of the 10,398 new case-builds, 1,304 differ
  from native. #734 holds 1,226 of them (a closure given by value dropped
  after what's left of its iterator), and #739 holds 14. Three new P0s
  were filed: #752 (a chain's `nth_back` never drops what it skips, 16),
  #753 (`--std-iterators`: a `step_by`'s `rfold` drops its closure twice,
  22) and #754 (a `rev().rfold()` whose closure panics never drops it,
  26). Within the hour, the fixes of #734 and #754 (1c23afa16, b0f7dfa22)
  took 1,063 of them off, #753's 22 among them. The slice's next run found
  #757, a regression of 1c23afa16: a chain the crate steps from its back,
  folded with a closure that drops, never drops its first side. 242 of
  the variants' case-builds stay listed.
- **Proved:** with #721's fix reverted, the slice fails on
  `plain: range | rev | count | none | after rfold` and its neighbours.
  With #719's fix reverted, the capturing cases are refused. Without the
  pairs, a cover of 61 plain batches passed with #721's fix reverted.
- **The conformance table's too** (ADR 1092), whose per-push 8 batches it
  replaces: `slice()` of its 360 batches is 43, `bun
  scripts/iteratorSlice.ts run conformance`, in 4 parts.
- **Its cost:** on the M3 Max, in 4 parts at a load of 14 to 20, with the
  debug compiler, the matrix's 64 batches took 119 to 145 s and the
  conformance table's 43 took 98 to 119 s. On CI, in 4 parts after a 116 s
  release build, 89 s and 62 s (run 41969266534).
- **Each in a job of its own**, the user's choice of 2026-11-05, beside the
  corpus with std's iterator structs (ADR 0908, amended): `iterators`,
  `iterator-conformance` and `iterator-forms`, about 5 minutes wall each,
  at once, beside the suite.

## Amendment: a change runs the rows it names before it lands (#818, 2026-11-06)

Most of the iterators' regressions were one fix breaking another adapter
or consumer: 15 of 23 since 11-04, three in one day (#823, #833, #834).
The fixed slices didn't sample the rows they broke, and the nightly's
full run, some 17 minutes of 12 cores, found them after they landed.

`check:fast` now runs, beside the slices, the rows of both tables a
change names (`bun scripts/iteratorSlice.ts run rows`, the picking
`scripts/iteratorRows.ts`'):

- **What a hunk names.** Each hunk the change makes to `iteratorPaths`,
  with the head of the function it's in and the arm of a `match` it's in
  (whose pattern names what its body is of, though that line is the same,
  #850's `IterMethod::Cloned`), names the generator's adapters
  and consumers, of its code, never its comments: adapters by its
  identifiers, their words inflected (`$taking` is `take`, `StepBy` is
  `step_by`); consumers by what it calls or names by path, `.nthBack(` or
  `IterMethod::Count`, and its function's name, from their first word,
  never a variable's or a parameter's (#849: a hunk to capacity.js named
  `count` and `for` of its sizes' variables and its comments, some 17,000
  rows, and took 1,800 s; now it names none and runs `pairs`, each
  destination among them). The runtime's generated module
  and the tests name nothing. The names are the generator's own lists, so
  there is no second catalog; picking is no compiler decision, so names
  are enough (m6-react's decision, for the user).
- **What runs.** Every row of each adapter a hunk names, whatever its
  consumer; of a hunk that names no adapter, every row of each consumer it
  names; of `$given`, generic code's, every generic source's. The rows of
  an adapter a hunk takes away more of than it puts back always run whole,
  whatever their count: removing code as equivalent decides for every row
  of what it names (#823). The rest share one budget for the change, the
  rows its parts run in some 2.5 minutes of the matrix's and 40 s of the
  table's at the machine's own rate (`budgetOf`: the median rows a second
  per part of its last runs, `target/iterator-rows-rates.json`, the M3
  Max's 9.2 and 10.8 until it has some; 16,560 and 5,184 rows there in 12
  parts, about 1,000 of each on the M1 in 4, #857): the shared sample's
  fifth of it reserved, then what's named, then the sample, a fifth of it. What's past what's left is `pairs`
  of it, as many of each pairing of an adapter with a consumer as fit,
  said so, with the setting that runs each.
- **What every adapter shares.** A hunk in the runtime's protocol
  (`$forward`, closing, consuming, `$given`, `$knowing`, `$held`,
  `$iterator`) or the lowering's iterator value, or that names none, runs
  `pairs`: 60 of each adapter with each consumer of the matrix, 10 of the
  table's, 200 and 103 batches.
- **The harness.** `ITERATOR_MATRIX` takes `/<regex>/`, the cases whose
  ids match, rebatched by 60, judged each by its number as in a full run;
  and `pairs[:<n>][/<regex>/]`. The batches holding them would be nearly
  all, as each batch holds every consumer.
- **What's said.** `check:fast` prints each hunk, what it names, and each
  run's rows and setting, so a miss is traced; its step is timed against
  its usual as each is.

Measured on 12 cores (2026-11-06): #834's change picks 10,353 and 4,347
rows, 111 s; the `step_by` rows alone, 13,400 of the matrix's, 123 s;
#810's `$givenDrop` arrow reintroduced picks the generic rows and `pairs`,
309 s, and fails in `pairs`' `zip.map` and `zip.filter` rows, as it
should. Replayed, 766055774, which made #823, runs every `enumerate` row,
#823's 19 among them, a removal; 731ae66fe, which made #834, names 60,505
matrix rows, past the budget, and runs 60 of each pairing of them, 10,329
rows, which hold some of #834's 34 about four times in five (#856's
budget: 31,676 rows of a removal and `pairs` took 868 s inside
check:fast, past its 600 s cap). The whole of a change's names is the
setting the report prints, for a fix that must run each. `test/iterator-rows.test.ts` holds the naming.
The pairwise comparison of the two builds the issue asked for isn't
made: each is compared with native Rust, which a disagreement between
them can't pass.

## Amendment: every point a panic can be, the points table (#817, 2026-11-06)

The user's decision, through m6-react: the matrix's panic sites were five
hand-picked places, the source's `next` at its third item, the chain's
first closure and the consumer's at their second call, and `D(3)`'s drop.
Most of the iterator P0s filed since 11-04 were one shape, what a step
drops as a panic unwinds from somewhere else: the first call, a passed-over
item's drop, the source's own drop, a closure's capture.

- **A third table, `ITERATOR_TABLE=points`** (`pointsCases()`), of the
  conformance table's cases that panic nowhere, 6,248 in 105 programs, with
  ids of the same `source | chain | consumer | site` shape, its site
  `points`.
- **Each case runs once a point** (`run_points`): under `catch_unwind`, its
  `k`th point panics, for `k` from 1 until a run has no `k`th, which is the
  case run plain. A point (`point()`) is each call of a closure of the
  case's, each drop and clone of a `D`, each drop of an accumulator or a
  total, and each `next`, `next_back` and drop of the crate's iterator. So no
  position is chosen by hand, and a new step's points are its own. One panic
  a run, so native Rust never aborts on a second (as rust-js would too,
  since #847; ADR 0660, amended).
- **The matrix's and the conformance table's programs are as before**, byte
  for byte, their generator versions too: the points are added to the
  prelude and the crate's iterators of a points program alone
  (`withPoints`).
- **When it runs:** every case each night, nightly.yml's `points`, one
  machine of 4 parts. All of it took 48 s on the Mac in 10 parts. Whether a
  push runs a slice of it is #818's gate's to decide.
- **What it found at once:** 166 of its 12,496 case-builds differ from
  native Rust. Four P0s were filed, each listed under its issue:
  #843 (a collect in place whose rest's drop panics leaks what it
  collected, both builds), #844 (a `skip()`'s passed-over item whose drop
  panics leaks the accumulator and the rest), #845 (a `step_by()` folded
  by value closes its source inside a step, so its drop's panic drops what
  rustc leaks) and #846 (a closure's captures stop dropping at the first
  whose drop panics, both builds).

**What rebatching finds (#850).** A row the gate runs is judged in a batch
of the rows it picked, not the full run's, so a row whose JS depends on
the rest of its crate, a type made stepped from its back by another
function, can run otherwise there. That's a wrong answer the full run's
batch happened to hide: `vec iter | chain.cloned | rfold` beside `chain.map
| rfold` threw on `main` (#850). A failing row the change can't explain is
bisected on a clean build of `main` first (`main`'s worktree, built), its
batch narrowed to the fewest rows that keep it wrong; one that fails there
too is filed or fixed before the change lands.


## Amendment: panic points beyond iterators, the unwind table (#854, 2026-11-07)

The user's decision, through m6-react: #817's points found four P0s at
once, and drops outside iterators had no such net.

- **A fourth table, `ITERATOR_TABLE=unwind`** (`test/unwind-points.ts`),
  run by the same harness, runner and oracle: `point()` and `run_points`
  are `test/panic-points.ts`'s, one module, and the harness judges any
  table through its `TableOf` (its cases, program, batches and slice),
  each case a `Row`. The iterator tables' programs are as before, byte for
  byte.
- **Its space is a table, family × operation × shape × point:** 16
  families (a value's fields and temporaries, closures that hold what
  drops, `Vec`, `VecDeque`, `BTreeMap` of values and of keys, `BTreeSet`,
  `HashMap`, `BinaryHeap`, `Option`'s and `Result`'s combinators of either
  variant, `?` failing and passing, comparisons), each operation of each of
  11 shapes it applies to (a `D`, a struct and an enum with and without a
  `Drop` of their own, a tuple, `Box`, `Option`, `Vec`, `Rc`, a closure
  holding a `D`): 3,470 cases in 58 programs. An operation says what its
  shape must have (`Clone`, `Eq`, `Ord`, `Default`); nothing is skipped by
  a list. A point is each make, drop, clone and comparison of a `D`, each
  drop of a struct or an enum with its own `Drop`, each call of a closure
  or of a fallible step; so the order the crate's `Ord`, `Eq` and `Clone`
  are called in is compared too.
- **No case panics but at its point.** A natural panic, an `unwrap` of
  `None` or `clamp`'s bounds out of order, unwinds past later points, and
  one of those panicking aborts native Rust's whole program (#847); the
  harness fails a batch whose native run doesn't end. `unwrap` is of `Some`
  and `Ok` alone, and `clamp`'s bounds are in order in every shape.
- **A `HashMap` holds one entry at a time**, so no output depends on its
  order, which native Rust's `RandomState` picks a run.
- **A batch whose every case is refused fails**: that's its prelude refused
  (a generic function over a `BTreeMap<T, _>` was, on the first run, and
  all 58 batches passed).
- **When it runs:** every case each night, nightly.yml's `unwind`, one
  machine of 4 parts. All of it took 27 s on the Mac in 10 parts. No push
  runs it yet: #818's gate decides.
- **What it found at once:** 591 of its 3,470 cases differ from native
  Rust in both builds, 1,182 case-builds, under 16 P0s, #897–#912, each
  listed in test/iterator-matrix-known.txt; 106 cases a build are refused.
  Five need no panic: `HashMap::retain`'s drop of a handle (#905),
  `VecDeque::retain_mut`'s eager drops (#909), `Rc`'s `==` by pointer
  (#910), and the order a stable sort (#907), a keyed `BTreeMap` (#911) and
  `max`, `min` and `clamp` (#912) call the crate's `Ord`.

## Amendment: every format spec of each value, the formatting grid (#941, 2026-11-08)

The user's decision, through m6-react: the generated tables found most of
the wrong answers, and `format!`'s output, the most visible a program
has, had no table.

- **A fifth table, `ITERATOR_TABLE=format`** (`test/format-grid.ts`), run
  by the same harness and oracle through its `TableOf`, each case a `Row`.
  Its runner is its own two lines, `run(name, case)`: a case never panics,
  so it needs no points.
- **Its space is spec × value × sink:** 60 specs of the eight traits a
  placeholder formats by (width, fill and alignment, a fill that isn't
  ASCII, `+`, `#`, `0`, precision, each given by argument too, `{:1$}`,
  `{:.*}`, `{:>w$.p$}`, `{:>1$?}`; `{:?}`, `{:#?}`, `{:x?}`; `{:x}`,
  `{:X}`, `{:o}`, `{:b}`, `{:e}`, `{:E}` with their options), of each of
  214 values of 53 types whose type has the trait: every integer type at
  its bounds, 0, 1, 42 and -1, and values whose exponent's precision
  rounds a tie or carries; `f32` and `f64` at their bounds, ±0, the
  infinities, NaN, subnormals, ties of `{:.2}` and `{:.0}`, and the
  exponent's thresholds of `{}` and `{:?}`; `char`s and strings with
  escapes, wide and combining characters, a leading combining mark,
  no-break and soft hyphen, private use and the last scalar; `bool`, `()`,
  `Option`, `Result`, tuples, arrays, slices, `Vec`, a `HashMap` and a
  `HashSet` of one entry, structs and enums with derived `Debug`, the
  crate's own `Display` that pads (`f.pad`), asks `f.alternate()`, passes
  its options on, writes past them, pads its own placeholders, and says
  what its `Formatter` was asked; its own `Debug` by each of `Formatter`'s
  builders; and `&`, `Box` and `Rc`. Each case writes its value bracketed,
  `[{:>7}]`, by `println!`, `format!`, `write!` into a `String`, a
  `panic!`'s message and `format_args!` given to a function, in turn:
  9,146 cases in 153 programs.
- **`isize` and `usize` are wasm32's, 32 bits** (ADR 0090), where the
  native oracle's are 64: their bounds are written as wasm32's,
  `0x7fff_ffff_isize`, and a negative one isn't written in a radix, whose
  digits are as many as its width. Nothing else is left out by a list.
- **When it runs:** every case each night, nightly.yml's `format`, one
  machine of 4 parts; and its slice, 22 fixed batches covering every spec,
  every type and every sink, per push whose change is the formatting code
  (`formatPaths` in `scripts/iteratorSlice.ts`), check.yml's `format-grid`
  and `check:fast`'s job, as #818's gate runs the iterators' slices. All of
  it takes 32 s on the M1 in 8 parts; the slice 4 s.
- **What it found:** no wrong answer: each of its 18,292 case-builds
  printed what native Rust does. One refusal, a precision of a `bool`'s
  `Display` or `Debug` or of `()`'s `Debug`, `{:.2}` of `true`, which std
  writes by `f.pad` and so cuts, `tr`, is now written (ADR 0058, amended).
  A break of `$zeroPad`'s sign fails 70 of the slice's case-builds.

## Amendment: every number operation of each type's boundary values, the numbers grid (#940, 2026-11-08)

The user's decision, through m6-react: after the formatting grid (#941),
numbers, where JS and Rust differ most, get a table.

- **A sixth table, `ITERATOR_TABLE=numbers`** (`test/number-grid.ts`), run
  by the same harness and oracle through its `TableOf`, in programs of 200,
  as each case is one line: 416 programs, not 1,400.
- **Its space is operation × type × value × second operand:** of each of
  the 12 integer types, `+`, `-`, `*`, `/` and the `div`s, `%` and the
  `rem`s, `div_euclid` and `rem_euclid`, `pow`, `<<` and `>>`, each in each
  of its forms (plain, `wrapping_`, `checked_`, `saturating_`,
  `overflowing_` where std has it), the rotations, `&`, `|`, `^`, `!`,
  negation, `abs` and its forms, `abs_diff`, `signum`, the bit counts,
  `swap_bytes`, `reverse_bits`, the powers of two, the `ilog`s, `isqrt`
  and `to_string`; of `f32` and `f64`, the arithmetic and `%`, the
  `_euclid`s, `copysign`, `mul_add`, `powi`, negation, rounding, `sqrt`,
  `signum`, `recip`, `to_bits`, `to_string` and each `is_` and `classify`;
  `as` between every pair of the 14 types; and `parse` and
  `from_str_radix` (2, 8, 16, 36) of strings that overflow by one, carry a
  sign, an underscore, a space or no digits. The values are each type's 0,
  ±1, ±2, MIN, MIN+1, MAX-1, MAX and those about 2^31, 2^32, 2^53 and 2^63
  it holds; a float's ±0, ±inf, NaN, a subnormal, `MIN_POSITIVE`,
  `EPSILON`, ties and the integers about 2^24 and 2^53. 83,017 cases, each
  printed by `{:?}`, or its panic's message.
- **Overflow wraps, as a release build's does** (the guide's semantics), and
  the oracle is built so, `-Coverflow-checks=off`; what panics in either is
  what a release build keeps, a division by zero, `MIN / -1`, an `ilog` of
  zero, whose messages are compared.
- **`isize` and `usize` are rust-js's, wasm32's** (ADR 0090), and the oracle
  holds them exactly: a case writes them `Isz` and `Usz`, `isize` and
  `usize` where `target_pointer_width` is 32, and `i32` and `u32` where it
  isn't, native Rust's. Each value is given through `black_box`, so rustc
  folds none of them.
- **The cases are interleaved** by a hash of each id, so each program holds
  many operations of many types, and the slice, the fewest whole batches
  that hold each operation, each type and each operation of each kind of
  type, is 7 programs.
- **When it runs:** every case each night, nightly.yml's `numbers`, one
  machine of 4 parts; its slice per push whose change is the numbers' code
  (`numberPaths` in `scripts/iteratorSlice.ts`), check.yml's `number-grid`
  and `check:fast`'s job. All of it takes 81 s on the M1 in 8 parts; the
  slice 3 s. A `$divEuclid` that didn't step a negative quotient down
  failed the slice.
- **What it found:** one wrong answer, #960, an `f32`'s `div_euclid`, whose
  quotient std rounds to an `f32` before it truncates it and the JS
  truncated as an `f64` (ADR 0064, amended); and one refusal, 2,216 cases a
  build, `overflowing_div_euclid` and `overflowing_rem_euclid`, now
  written. Each of its 166,034 case-builds now prints what native Rust does.
