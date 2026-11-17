# 1101. The inner check runs what a change reaches

Status: Accepted (#877). Amends [1093](1093-the-check-before-a-push-is-the-pushs-check.md):
plain `check:fast` is the inner loop's check, `--all` still the check
before a push and unchanged. Records with
[0093](0093-mutations.md)'s kind of proof in reverse: what each test runs
of the compiler, not what each mutation breaks.

Case ([0262](0262-when-rust-and-js-disagree.md)): none; which tests the
inner loop runs, not what the JS means.

## Context

Plain `check:fast` runs after each edit. Of a change to `src/` it ran the
corpus in 12 parts and nine suites by the change's paths, each suite one
`bun test` of its files one after another: lowering's 74 files 70 s, the
last job of every run, the cores half idle 44% of it. On the 12-core M6 a
one-statement change to tidy took 79 s, to drops 85 s, to the bindings
77 s, to the iterators 153 s and to the runtime 136 s, these two with the
iterator rows they name (2026-11-07).

The suites ran whatever the change's paths said, the same for a change
to a function every compile runs as for one only a closure's capture
reaches.

## Decision

**Each plain suite's file is a job of the pool's**, as each file is in
`--all`: the suites (`scripts/shard.ts`'s `plainSuites`, the one list)
run side by side, said as their suite.

**A test map says which tests a change can affect.** `bun run test-map`
records it of HEAD (`scripts/testMap.ts`): a build of the compiler that
counts each function's runs (`-C instrument-coverage`), given to each file
of the plain suites and each corpus case, through a stand-in
(`scripts/testMapCompiler.ts`) that keeps the functions each compile ran,
a bit each, for the test it ran for: a corpus compile for the case its
wrapper includes. Kept by its commit, for every worktree of the machine
(#932, below).

Plain `check:fast` reads the newest map of HEAD or an ancestor and the
working tree's difference from its commit. Where each changed line is a
function's counted code, it runs the files and cases that ran one of
those functions, and those that run on every change; a changed plain test
file or corpus case runs itself, and a case the tests that read the
corpus's files. Anything else and it runs what the paths say, as before,
and says why.

## Why a skip is sound

A test runs the compiler's code; a function it never ran, changed, can't
change what it runs, so it ends as it did at the map's commit. That holds
only for a change to a function's code, and only where the test runs
nothing the map didn't count. So:

- **A changed line must be counted code of a function** (llvm-cov's code
  regions), joined over what's the function's own between them: a `use`,
  a closing brace, a comment, a pattern. A type, a constant, an attribute,
  a derive, a line between two functions, a `#[cfg(test)]` module: the map
  can't say. Lines added between two must both be one function's code, or
  of one nested in the other, a closure in it. A change that writes an
  item (`impl`, `const`, `static`, ...) inside a function can't be said
  either: an `impl` nested in a function is used outside it.
- **A file `include_str!` reads** is the line that reads it.
- **Only `src/`'s code.** A test's helper, std, the runtime's package,
  Cargo's files, the toolchain, a doc a test reads: the map can't say. So
  any of those changed since the map's commit, committed or not, and the
  check runs what the paths say until the map is recorded again.
- **Each test records alone**: its own `RUST_JS_TARGET` and std sysroot,
  and no compile cache, so a crate one test built isn't read by another
  unrecorded; the corpus's native builds and verified runs, which no
  compile of rust-js's makes, are shared. The native builds are the
  checkout's in any target (ADR 0104, amended, #875): kept by a path in
  the test's own target, each recording built 3,642 again, and left them.
- **Every run is counted, or the test runs always.** The compiler runs
  itself again for each crate of crates.io it tries: each process writes
  its own counts (`%p`). A run given no `LLVM_PROFILE_FILE` writes its
  counts where the build says (`__llvm_profile_filename`), and the
  recording fails. A compile stopped or aborted, which writes none, a
  function the table doesn't name, a test that failed, or one that ran no
  compiler, runs on every change. So does one that reads the compiler's
  sources as text (`readsSources`: the architecture's tests, the mutation
  lists).

## Measured

On the 12-core M6 (2026-11-07), five one-statement changes, plain
`check:fast` each, a statement added to tidy's `guarded_chains`, an
iterator's `collects_in_place`, drops' `glue_place`, the runtime's
`helper_of` and the bindings' `is_nullable`:

| Change | Before | Each file a job | With the map | What the map picks, of 101 files and 1,809 cases |
|---|---|---|---|---|
| tidy | 79 s | 57 s | 60 s | 101, 1,805 |
| iterators | 153 s | 144 s | 83 s | 7, 62 |
| drops | 85 s | 63 s | 59 s | 88, 1,231 |
| runtime | 136 s | 123 s | 121 s | 80, 1,538 |
| bindings | 77 s | 53 s | 56 s | 97, 1,631 |
| together | 530 s | 440 s | 379 s | |

The iterators' and the runtime's runs are the rows they name of the
iterator matrix too, which the map doesn't choose (#818). Each file a job,
the cores are 93–95% busy, so what's left is the CPU of what runs: a
function every compile runs, tidy's, picks all of it.

Of the catalog's 9,070 mutations whose own tests are the plain suites' or
the corpus's, the map can say for 8,981 and picks each one's own tests for
all of them: of an iterator's, a median 25 files and 236 cases; of drops',
44 and 500; of tidy's and the runtime's, nearly all. A deliberately broken
module of each area failed plain `check:fast` with the map: an iterator's
`extend` (20 files and 37 cases picked), a closure's drop (12, 65), tidy's
inlining (all), the runtime's `helper_of` (80, 1,538) and its `$at` (all),
a binding's keyed test (9, 4) and a WebIDL bound (14, 0). Of each but
tidy's, every plain file and every case was run too without the map:
each one that failed was one the map picked.

## Alternatives

- **By module, from `use` or the call graph**: every module of a compiler
  reaches nearly every other; nothing would be skipped.
- **Line coverage, not functions**: a test that took another arm of a
  `match` ran its patterns, which coverage counts as the arm's; a change to
  a pattern it never matched could make it match. A function it entered is
  the unit whose change can't reach a test that never entered it.
- **Recording in every plain check**, the map kept current: the counting
  build is slower, and each run's records would mix commits.
- **A map for a change to `docs/` or a test's helper**: what a test reads
  of them isn't counted; Bun's `node:fs` can't be watched from a preload for
  the ESM imports the tests use.

## Consequences

- `bun run test-map` after `git reset --hard origin/main`, and after
  committing a change outside `src/`: about 130 s on 12 cores.
- The map's build, `target/test-map/build`, is kept for the next
  recording, one build as `target/debug` is; the maps, three, each about
  12 MB, the least recently used going first, the machine's since #932.
- `--all` is unchanged: what a push's check runs.

## Amended: the corpus with std's iterator structs, by case (#876)

A change to the iterators' code runs the corpus built with std's iterator
structs (`ITERATOR_FORMS=corpus`, scripts/iteratorSlice.ts), every case,
29 s at 4.7 cores of plain `check:fast`'s 83 s for an iterator change. The
map records that build too, each case a test of its own
(`caseUnit`: `test/iterator-forms.test.ts test/corpus/<case>.rs`), as
its compiles take other paths than the plain build's, so the plain build's
picks can't stand for them. Plain `check:fast` runs the cases it picks
(`run forms <parts> <list>`), none where it picks none; a change to the
file runs each, and to a case, that case too. `--all` runs every case, as
before.

- The iterator change of the five: 83 s, now 73-74 s twice, the job 29 s,
  now 2 s (74 of 1,809 cases); recording 132 s, not 110 s.
- A closure's drop broken: 46 of the build's cases fail run over every
  case with no map, each of them among the 81 the map picks; an
  iterator's `extend` and an `Option`'s `then` broken fail none of them,
  of 54 and 397 picked.
- What's left of an iterator change is the rows it names (#818), 59 s of
  the 74, sized to their budget of time, and the two fixed slices, 16 and
  10 s. Of `--all`'s about 2,270 CPU-seconds with an iterator change, the
  rows are 385, the build with std's structs 160, the slices 128; run
  again with the same build, 124, 13 and 30, the compiles kept (ADR 1099)
  and the native programs too. The rows the two tables share: 2,235 of
  the conformance table's 21,594 cases are the matrix's of the same code,
  but in other batches, whose make-up changes a compile (#826, #835), and
  the table's budget is 40 s: at most 4 s, not taken.

## Amended: the maps are the machine's (#932)

Each worktree recorded its own maps, in its `target/test-map/maps/`, and
each task starts in a fresh one: `bun run test-map` took 261 s there on
the 12-core M6 (unwindD), 46 s of it the worktree's counting build, though
a commit's map is the same in every checkout of it.

- **One store for the machine's worktrees**, `~/.cache/rust-js/test-maps/`
  (`mapStore`), outside any `target/`, so `cargo clean` keeps it. A map's
  file is named by its key, its commit, the toolchain's commit hash
  (`rustc -vV`'s, its `rust-toolchain.toml`'s) and the layout's version:
  a check reads only one of its own toolchain and layout.
- **A check reads the map of HEAD's nearest ancestor**, not the newest
  read: of maps another worktree recorded, one of a commit a few back
  says more than one of a commit far back read since. A commit the
  worktree hasn't, another clone's, is no ancestor of it.
- **`bun run test-map` of a commit recorded ends at once**, saying whose;
  `--keep` records again, its counts kept.
- **Written whole, then named**, so another worktree never reads half of
  one; one half written by a recording that stopped is never read, and
  goes as the least recently used do.
- **Bounded as before** (test/bounded.ts): three, the least recently used
  going first, none kept below the disk's floor, and a map of another
  toolchain or layout, a stale key, removed as a recording ends.

Why it's sound: what a map records is its commit's compiler and tests, run
each in a target of its own, the same in any checkout of the commit with
the same toolchain; a failed or uncounted test of one recording runs on
every change, which only picks more.

**The counting build stays the worktree's.** It took 46 s fresh: the
compiler's own crate 27 s, counted and incremental, which sccache never
keeps, and its dependencies 19 s. The recording gave it its target as
`CARGO_TARGET_DIR`, which sccache's key holds (rustc's environment's
`CARGO_*`), so no crate of it ever hit; given as `--target-dir`, 90 of
its 146 crates do, but not the 56 a proc macro builds or is, whose
dylibs differ by target, so it's 44 s still. A target the worktrees
share isn't sound: Cargo takes a path package's build as fresh by its
files' times, so a worktree whose files are older than another's build
of another commit would count that build's compiler. With the store, it's
built once a commit on the machine, not once a worktree.

Measured on the 12-core M6 (2026-11-08), a fresh worktree of a commit
another had recorded: `bun run test-map` 0 s, not 195–261 s (46 s the
build, then 149 s of the suite with the natives' builds kept, 261 s from a
cold store); its first plain `check:fast` of a statement added to drops'
`glue_place`, the build of `target/debug` included, 140 s, the map picking
91 of 104 files and 1,249 of 1,831 cases, where with no map it ran what
the paths say, 157 s. A task's start and first check: about 140 s, not
352–418 s.

Not done: a map recorded by CI of each push and fetched. CI's runners are
Linux's, whose `cfg`s compile other functions, so its map isn't the Mac's.

## Amendment: a test that reads a changed file as text runs (2026-11-09)

A test of a file's text fails on a change to the file that runs none of
its code: `ci.test.ts` counts the crate corpus script's Cargo runs, and
a9aedb875 moved one onto `test/child.ts`'s `run`, which it didn't count. The
map couldn't say of a change to `scripts/`, the paths picked the plain
suites, and `ci.test.ts`, outside them, didn't run: `main` was red until the
next `--all`. **Of a change, each test file whose own text names a changed
file runs too, whatever the map or the paths pick** (`readersOf`, and
check:fast says `<test> reads <file>: it runs`). What a test reads is what
its code names, `"scripts/crate-corpus.ts"`, `join(root, "src")`,
`${root}/std`: a literal names a file, a folder only of the checkout's
root, and a file with a `root` of its own, a fixture's, names none of the
checkout's; comments aren't read. Its helpers' names aren't its own, as each
names what every test reads as it runs, `std/`. Bun binds a test's `node:fs`
as it links it, so its reads can't be seen as they're made, and this is of
its source. `readsSources`, a test that reads the compiler's sources, runs
on every change, is the same reading followed into its helpers, the mapped
files' answers as they were. Held by `test/test-map.test.ts`'s "a change to
a file a test reads as text picks the test".

## Amendment: one selector, for `bun run push` and CI's suite (2026-11-16)

Tu's direction: test only what a change can affect. A push ran `check:fast
--all`, about 4–5 min here, and CI's suite every file and case on two
machines, whatever the change was. Four agents on four machines push to
`main` every 15–30 min, so most of that runs tests a change can't reach.

**One selector, `scripts/select.ts`, answers which test files and corpus
cases a change can affect**: `{all, files, cases, forms, why}`. Plain
`check:fast` runs what it selects, which `bun run push` runs, the same check
again of the new tree past a rebase that brought commits (`sanity`); CI's
`plan` selects too, and its test shards run that (`bun run test
--selection=`, the corpus by `RUST_JS_CASES=i/n:<file>` as before). `--all`
stays everything, and the nightly runs everything.

Its rules, by the changed file's path:

| Path | Runs |
|---|---|
| `src/**`, a file `include_str!` reads | the map: the tests and cases whose recorded functions it touches; `all` where it's no function's code |
| `std/**`, `src/runtime/**` | its package tests, every case, every file the map saw compile (not those that ran no compiler) |
| `test/<x>.test.ts` | itself; the corpus's file, every case |
| `test/corpus/<case>.*` | the case, the map's corpus readers and the baselines' owners |
| the workflows, the packages' manifests and lockfile, the toolchain, Bun's config, the selector and the map | everything |
| a package's or crate's folder (`webapi/`, `react/`, `vendor/`, `tooling/`, …) | everything: tests build them as apps' dependencies, which no import or read says |
| any other (`scripts/`, `test/`'s helpers and data, `docs/`, root docs) | each test that imports it, transitively, Bun's preload `test/setup.ts` every test's, or reads it as text, itself or through a module it imports (`readsOf`, `join(import.meta.dir, …)` too); none where none does |

Baselines a test reads by a path its source doesn't spell are listed with
their owners in the selector (`baselines`).

**The map records the whole push suite**, every file but the nightly's,
not the plain suites': 355 files, the metamorphic sample in parts as the
suite runs it, and of each file that ran no compiler, `noCompiler`. The
plain suites' list, `plainSuites`, is gone.

### Why it's sound under four agents pushing

The diff is the map's commit `M`'s to the working tree, one diff. It holds
others' commits since `M` (`M..B`, `B` the branch's base) and the branch's
own (`B..` the tree). A test is skipped only where nothing it reaches
changed in either: no line of a function the map recorded it running, not
itself, no module it imports and no file it reads. Then it runs at the
working tree exactly as at `M`, where the map recorded it passing (one
that failed, didn't finish, ran no compiler or read the compiler's sources
is `null`, run on every change to the compiler). So staleness is per test,
not per map: a test whose functions another agent's commit touched runs; a
commit that makes test T reach a function f of this change touched one of
T's recorded functions (the call to f is in one), so T runs. Only a change
the map can't place, a line outside any function's counted code (a type, a
constant, an item) or a new file of `src/`, runs everything, and the
selection says whether that change is the branch's own or others'
(`allOwn`), on every push's line and in CI's summary, which count how
often each forces it.

A red or still-running parent needs nothing more: CI diffs against `M`,
never the parent, so a test red at the parent `P` was broken by a change
in `M..P`, which is in `M..HEAD` too, and runs again (unless reverted, and
then it passes). No failure is carried over from run to run. The hole: a
test red with no change behind it, a flaky one's or the environment's. The
nightly runs everything, and its audit (`scripts/selector-audit.ts`) asks
of each test file it failed whether any push's selection since the last
green night ran it; a miss is filed as a P1 `selector: …`.

**Unsure means more**: no map of HEAD or an ancestor (a change to the
compiler, std or the runtime then runs everything; one that needs no map,
a doc's or a script's, still runs its readers), a stale line, a new file of
`src/`, a map of another layout or toolchain, or a change to the selector
or the map.

### One OS's map serves the other

CI records the maps on Linux; `bun run push` reads them on macOS. That's
sound because each test runs the same compiler code on both: `src/`
branches on no OS but WASI, and the harness's OS branches each change
nothing a test compiles (`osBranches`, with why), both held by
`test/select.test.ts`. A new branch is listed with its reason, or the maps
get keyed by OS.

### Fresh maps

`test-map.yml` records HEAD's map after each check of `main` completes, red
or green, in a run of its own (`workflow_run`), so it never takes a machine
from a check's jobs; one at a time, the running one never cancelled, and of
those waiting only the newest kept, as GitHub replaces a pending run of the
group with the next. (Cancelling the running one, as it first did, starved
it: checks end about every 10 min and a recording takes longer, so every
run was cancelled and none was recorded for 9 commits.) Each map is cached by
its commit, one per entry, the newest 8 kept (`cache-prune.yml`, about
6 MB each; amended 2026-11-17: one prune keeps every cache family's newest
8, the binaries' and native stores' too, `scripts/ci.ts`'s `cacheFamily`), and uploaded as `test-map-of-<sha>`, which `bun run push`
downloads where it's nearer than the machine's (`fetchMap`; of this
repository's `main` runs alone: test-map.yml's last runs, each asked for
its own artifacts, as the repository's list of all of them, 100 a page,
held one past its first page). A map not fetched says why on the push's
line, `[no map fetched: ...]`, a `gh` not logged in among them. rustc-tests.yml's mutations call the same
workflow. The machine keeps 8 maps (`keptMaps`, scripts/disk.ts's LRU).

### Measured

On the 12-core Mac mini, the map of the whole push suite recorded in
598 s (the counting build 55 s; 22,643 functions, 5,655 tests; 157 run on
every change: 40 read the compiler's sources, 73 run no compiler, 5 failed
under the recording, the rest cases with std's iterator structs that run
none), 22 MB. Each run here its own processes alone, the load 9–15 from
them (2026-11-16):

| Change | Before, `check:fast --all` | After, the selection |
|---|---|---|
| A line in a function of `src/lower/std_types/slice.rs` | 353 s | 214 s: 94 of 356 files, 71 of 2,649 cases; the nets its paths run are most of it (the twin net 101 s, the crate corpus 69 s) |
| A script, `scripts/push.ts` | the same | 5 files, 21 s of their own; 152 s with the iterator nets, which the branch's change to `scripts/iteratorSlice.ts` ran |
| A doc nothing reads | the same | none: `bun run push` goes straight to the push |

Held by `test/select.test.ts` (each kind of change; the real suite's
imports and reads; check:fast's jobs; push's line; the OS branches; the
audit), `test/test-map.test.ts` (the merge of parted files, CI's workflow,
the fetch and the cache's pruning), `test/ci.test.ts`, and the mutations of
`scripts/mutations/select.ts`, each rule dropped.

### Amended: a sanity pass is the change's own (2026-11-16)

Past a rebase that brought commits, `bun run push` checked again by the
same selection: the map's diff to the new tree, which holds others'
commits. Their non-function lines made it select everything, so a sanity
pass took longer than the check: m3-max's took 594 s and 516 s after a
491 s check, and m1-lisa's 1,228 s. Her push gave up after main moved
three times (3,847 s, not pushed). Tu's rule: quick, then push; CI's full
check covers the combination.

**The full argument above holds for `check:fast` and CI's suite.**

**A sanity pass runs the interaction** (`check:fast --sanity=<the commit
checked before the rebase>`, the push's `sanity` phase, `interaction`): the
tests both the change and the commits the rebase brought can affect. The
argument for this, which the earlier "the change's own" pass lacked:

- A test that only the change affects ran with the change before the
  rebase. Its outcome can differ now only if the incoming commits affect it
  too.
- A test that only the incoming commits affect was checked by their
  pushers. Its outcome can differ only if the change affects it too.
- A test that neither affects keeps the outcome both checks saw.

So the pass is the intersection of two selections by the fetched map: the
change's own diff since its merge-base with `main`, and the incoming
commits' diff from the old merge-base to the new one, which counts as
others' (`ownRunsAll`'s files run only what reads them). A compiler file
the map's commit and a side's base differ on has no line of the map's, so
that side takes every test the map saw run any of its functions
(`unitsReaching`). Where one side selects everything, the pass is the
other side's. Where both do, it's the cap below. Past `sanityBudget`'s 40
test files, about five minutes on the Mac mini, it's the interaction's
corpus cases alone, `capped corpus of the interaction`. m1-lisa's runtime
change (#1976), "some (360 of 361 files, every case)", ran all of it again
after each rebase, about 1,120 s on the M1. It lost to a moving `main`
three times, until her push's 4,200 s bound stopped it.

Where both sides select everything, the pass is capped. It runs the
change's own test files and corpus cases, with no lint, nets or builds
beyond the push's own build and std. The line says `capped`. **A cap is
never empty for a change to the compiler's sources** (#1988): it takes the
tests and cases the map, as of its commit, recorded running any function
of the files the change touches, an approximation, `capped some (from the
map at <sha>: N files, M cases)`; or, where that's none, the whole corpus,
`capped corpus`. Two changes to one file, the change's and others', are
where they most meet: m6-lily's tidy refactor's cap ran nothing. Others'
commits were checked by their own pushes, and CI checks the combination.
With a pass this cheap, a push that meets a moved `main` rebases and runs
it again until it lands, with no limit.

**What changes which tests run, or how CI runs them, not what a test does,
runs everything only of the change itself** (`ownRunsAll`): `.github/**`,
as CI's run of it changes, and the selector and the map, `scripts/select.ts`,
`scripts/testMap.ts` and `scripts/testMapCompiler.ts`, as their rule may be
wrong. Of others' commits since the map's, each runs only what reads it.
The selector's own frequent pushes, a map 12 behind, made every agent's
check select everything, "scripts/select.ts is what every test runs under
(others' since the map's commit)".

Every push's line says the map it selected by and how far behind HEAD it
is (`map <sha> N behind`). It was often past 8: 10 at the median and 16
at most of the Mac mini's ten push lines of 2026-11-16, since the
recording started only once a check ended, 22–28 min, and took 10–17 min.
**So the map is recorded on each push to `main` that changes what a test
runs, beside the push's check** (test-map.yml's `push` trigger, `paths`
without `docs/**` and `*.md`). It's still one recording at a time, the
running one never cancelled and only the newest waiting one kept. The
check's suite takes one machine of a small selection, which leaves room
for it. The lag and the 20-job cap's minutes before and after are in
#1989. The line says who ran it
by `RUST_JS_MACHINE` and `RUST_JS_AGENT`, each machine's sessions' own
(`whoRuns`), else the host's name with a note. It's written on every exit,
including a signal's.

### Amended: a failed check says which commit broke what (2026-11-16)

A check of `main` is a batch: it runs once over the commits since the last
completed check, so a red one named no commit, and every agent with a
commit in it looked. **Where a check's suite fails, it attributes each
failure** (`scripts/attribute.ts`, check.yml's `attribute-plan`,
`attribute` and `attribute-report`). Each suite shard writes its JUnit
report, now of every run, not only the nightly's. The failing test files
and corpus cases run again at the batch's commits: each of them if six or
fewer, else six spread from the oldest to the newest. Each commit gets a
machine of its own, its own build and its own harness. The batch's base is
the newest check of `main` that completed, red or green (`plan`'s
`batch_base`). The first commit each failure fails at from there on broke
it. A failure that fails at the first commit is said "or before", and one
that passes at the newest is said "flaky or the check's machine's".

The lines, `<test> broke at <sha> (<subject>)`, are in the run's summary,
in an `attribution` artifact, and under the red run in `bun run ci:status`.
Nothing is posted to a commit: a commit comment notifies its author's
account, which is Tu's. The jobs run only after a failed suite, so a green
check costs nothing more. Proof: `test/attribute.test.ts` and the
`attribute-*` mutations.

**A failed check's mutations are attributed too, by blame, with no rerun**
(check.yml's `attribute-mutations`, `scripts/attribute.ts mutations`).
Mutation survivors were the day's most common red `main`, and two red
checks named no commit (43871190255, 43872319768). Each mutation a shard
didn't catch, if it isn't latent, is said to be changed by the batch's
newest commit that touched its site's lines (`git log -L`), its list or
its tests; one nothing in the batch touched is said to be latent, from
before the batch. A shard, or the survivors' job, that failed with no
verdict is said to be broken by the batch's newest commit that touched
that job's own runner files (`runnerFiles`); where none did, by an earlier
change, its machine or GitHub. The lines are in the run's summary, an
`attribution-mutations` artifact, and `bun run ci:status`.
`bun scripts/attribute.ts mutations <dir> <base> <head>` says it of a past
run.

### Amended: every Actions minute counts (2026-11-16)

Tu's standing guarantee. It rests on two rules, kept in docs/dev-loop.md's
"CI spend".

**A nightly CI spend report.** `scripts/ci-spend.ts`, the nightly's
`ci-spend` job, keeps one comment on the pinned issue #1989. It counts
runner-minutes, share of the 20-job cap, queue waits, push-to-verdict
latency and wasted minutes, by workflow and by job kind, with the trend
against the days before. Waste is a cancelled run's jobs; a re-run's (a
later attempt, or a run of a commit its workflow already ran by the same
event); an unread run's (its verdict after a newer run's of its workflow
and branch); and a job that timed out or was stopped in a run that wasn't
cancelled. The report flags waste over 10% of the day, and a job kind of
30 minutes or more grown past 30% on the day before.

**The gate rule.** Each job of a push's check either has a gate, its `if`
reading `plan`'s outputs or another job's result, or is in `scripts/ci.ts`'s
`alwaysRun` with a one-line reason. Every other workflow a push starts has
a `paths:` filter. `test/ci.test.ts` fails on a job with neither.

Proof: `test/ci-spend.test.ts` and the `ci-spend-*` mutations;
`test/ci.test.ts`'s "each job of a push's check is gated" and the mutation
`always-run-reason-dropped`.

### Amended: a push's check is its own change's, never others' `all` (2026-11-16)

m1-lisa's test-only push selected everything: "src/js/writes.rs's change
at line 89 is outside any function's counted code (others' since the map's
commit)". The first check of a push took the whole diff from the map's
commit, so any other agent's item, type or new file since the map ran
every agent's whole suite until the next map. On the M1 it was most pushes.

**A push's first check is the change's own** (`pushSelection`, `Rule`
`push`, `bun run push` and `check:fast`):

- It runs what the branch's own change since its merge-base can affect, by
  the map. A file the map's commit and the merge-base differ on is taken
  as each test of its functions, as the sanity pass does.
- Where the change touches the compiler's code (`src/**`), it also runs
  what others' commits since the map reach by their functions' changes.
  Their commit may have made a test reach one of this change's functions,
  which the map at its commit can't say.
- Never others' `all`. Their item, type or new file was checked by their
  own push and by CI.

**What it leaves, and who holds it.** An item of another's that a function
neither side changed reads (a `const ENABLE` read by `h`), which makes a
test reach this change's function `f`: neither side selects that test.
CI's check of `main` keeps the full rule (`plan`'s `selectChange(since)`):
the whole diff from its map's commit, others' `all` inherited. So the
combination is checked there, on every push. `test/ci.test.ts`'s "CI's
plan selects by the full rule, the push by its own" holds both callers.

Proof: `test/select.test.ts`'s "a push's check runs its own change's
tests, others' function changes only beside its compiler code, never
their all":
- her case, her own test file beside another's item, selects her file
  alone;
- an own compiler change beside another's function change runs both
  sides';
- beside another's item, its own alone;
- its own item, everything.
Mutations `push-selection-inherits-others-all` and
`push-selection-drops-others-functions`.

### Amended: comments, new items and a signature's visibility select no more than they change (2026-11-17)

Every M1 handoff of 2026-11-16 selected everything for a line no test's
outcome can turn on: m1-lisa's heap split's `mod derived;` ("writes an
item"), #1983's `fn given_stepped` made `pub(super)` ("outside any
function's counted code"). A hunk now keeps its removed lines too, and
(`pick`):

- a hunk of comments or blank lines alone selects nothing;
- a pure addition outside any function's code, of items nothing written
  before refers to (a `fn`, a type, a `const` or `static`, a `mod`), selects
  nothing: a test reaches it only through a function changed to use it,
  whose own hunk selects that function's tests. An `impl` (whose methods
  are found where nothing names it, a new inherent method shadowing a
  trait's), a `use` (which may shadow a glob's name), a `trait` or a macro
  still selects everything;
- a function's signature line changed only in its visibility, or its docs,
  is that function's change, the function whose counted code follows it.
  A signature whose types change is still everything: its callers change.

Proof: test/test-map.test.ts's "a comment, a new item nothing refers to,
and a signature's visibility select no more than they change"; mutations
`selector-comment-runs-all`, `selector-pure-addition-runs-all`,
`selector-added-impl-unreferenced`, `selector-signature-visibility-runs-all`
and `selector-signature-types-as-its-fn`.

### Amended: a red forms or nets job names a commit too (2026-11-17, #2003)

Checks 43921007925 and 43925046527 failed only "iterator forms of the
corpus", `runtime_long_writes with std's iterator structs` past its 10 s,
and `attribute-plan`, which read only the suite's shards' JUnit reports,
was skipped: the cause, baa87ac92, was found by hand.

**Every slice run writes its JUnit reports** (`scripts/iteratorSlice.ts`'s
`run`): each part's, in `target/junit/slices/`, beside a
`<report>.json` saying what ran it, its run's name, file and settings
(`SliceReport`). `iterator-forms` and `nets` upload them, `junit-forms`
and `junit-nets`, as the suite's shards upload theirs, and
`attribute-plan` needs both jobs and runs where any of the three failed.

**Each slice run's failing tests are run again** at the batch's commits,
by its file, its settings and a `-t` of their names, every part's at
once (`sliceReruns`, failing.json's `runs`), each commit's own harness as
the suite's are. A run whose file isn't at a commit is said not to fail
there. Its line names the run and its tests: `the corpus with std's
iterator structs: runtime_long_writes with std's iterator structs broke at
<sha> (<subject>)`.

**A failed net no rerun can say is named by blame** (`unrerunnable`): the
metamorphic rewrites, whose cases are the change's own and differ at each
commit, and the crate corpus and React's port, which run no `bun test`.
Each is said `<step> failed; not attributable by rerun: last change to its
inputs was <sha> (<subject>)`, the batch's newest commit that touched its
inputs (the paths that run it, `cratePaths` and `portPaths`), as a
mutations job's runner is (`runnerFiles`); where none did, an earlier
change's, its machine's or GitHub's. Its failed steps are the run's, by
GitHub's API, in `attribute-plan`'s summary.

**And the slowdown itself is measured**: test/lend-bench.ts times a
`$lent` made each step, of a JS iterator and of std's compiled struct, the
best of five, reported and never asserted, as a loaded machine's times are
noise. The nightly's `ci-spend` keeps them in #1989's comment with the
days and flags one past 1.5 times the night before's.

Proof: test/attribute.test.ts's "a slice run's failing tests run again"
and "a failed net no rerun can say"; test/ci.test.ts's "each job that runs
a slice uploads its JUnit reports"; test/ci-spend.test.ts's "a
micro-benchmark past 1.5 times"; mutations `attribute-slice-*`,
`attribute-unrerunnable-unnamed` and `ci-spend-bench-unflagged`.

### Amended: a constant's value is its readers' change (2026-11-17)

m3-max's adapter-family flips each grew `HELD_FLIPPED: [&str; N]` by one
entry, and each selected everything: the declaration's line is outside any
function's code, and its type changed, its length. A constant's value is
inlined where it's named, so its change is read only by the functions that
name it. A hunk within a `const` or `static` item, from its declaration to
its `;`, whose type is the same but for an array's length, and which writes
no function, closure or other item, is now the change of each function that
names it (`git grep -w` at the map's commit, comments and `use`s left out).
A constant named where no function is, another constant's initializer,
still selects everything, as does one whose type changes otherwise.

On dab80f433's diff from the map at 1bfa70125: everything ("writes an
item, `pub(in crate::lower) const HELD_FLIPPED`"), 620 s on CI, before;
after, its one reader, in_place.rs:145, which every test reaches, so 364
files and every case: the rule is right, and this constant's reader is a
hub, which region coverage for hubs is to narrow.

Proof: test/test-map.test.ts's "a constant's value"; mutations
`selector-const-value-runs-all`, `selector-const-retyped-as-value` and
`selector-const-read-outside-kept`.

### Amended: a test reads what the helpers it imports read, so a helper of a few is a module of its own (2026-11-17)

A test's reads are its modules' (`reachOf`), and every test imports
test/support.ts: each path a helper there read was every test's. An edit
of docs/guide/limitations.md, which `limitations()` alone reads, selected
all 365 files (d0ce7d1ab's and 336483fa0's incoming side), as did a change
to node/'s or react-native/'s build scripts, test/ports or serde's build.
Each such helper of a few tests is a module of its own, imported by those
alone: `limitations()` (test/limitations-of.ts), `buildReactNative`
(test/react-native-crate.ts) and `ported` and its kin (test/ports-of.ts).
`buildNode` (test/node-crate.ts), `buildSerde` (test/serde-crate.ts) and
`cargoApp` (test/cargo-app.ts) followed, once the four test files at the
cap that import them were split by concept (#1334). What support.ts still
reads, react/'s build and the app seed's CLI, most tests do read.

### Amended: a new module of items nothing refers to selects nothing of its own (2026-11-17)

A new src file selected everything ("is new since the map's commit"), as
4ba38fa64's src/prepare/import_names.rs did on every rebase's incoming
side. Nothing written before refers to a new module's items: it's reached
through its `mod` line, a pure addition, and the functions changed to use
it, whose own hunks select their tests. So a new module selects nothing of
its own (`unreferencedModule`), but where it writes an `impl` of a type it
doesn't declare, whose methods may be found where nothing names it; a
`trait`; a `static`, or a `const` whose initializer calls or closes over
code; a macro, defined or invoked as an item; or a `#[test]`. Its own
types' `impl`s, a trait's for one of them too, are found only through a
value of the type, which only code changed to use it makes.

The interaction sanity's three examples (2905d5cdc, d0ce7d1ab,
336483fa0), recomputed with the map at 2905d5cdc and each commit's three
before it as the incoming side: 2905d5cdc's incoming side everything
before, 362 files after; d0ce7d1ab's and 336483fa0's, limitations.md 365
files to 5. Each interaction is still 291 to 362 files: both sides change
functions every test reaches (`lower_crate`, src/output.rs's), which the
hubs' region coverage is to narrow.

### Amended: a name no item declared, added to a `use`, is no change of its own (2026-11-17)

4ba38fa64 selected everything for src/js.rs:43, `imports::{Import,
ImportFrom, Package, Reexport}` given `ImportNames`, a type the same change
declared. A `use` stays everything where it may shadow a glob's name, which
only a name some item declares can: so a hunk within a `use` item that
only adds names (`newNamesUsed`), none of which any item declared at the
map's commit (`git grep -w`), changes nothing but the code changed to use
them. A name taken out, or one an item declared, is everything still.

4ba38fa64 from the map at afebe11b2, after: the next hunk selects
everything, `pub import_names: ImportNames` added to `Module`, which
derives `PartialEq`: a new field changes what equality compares wherever
it's asked, so a field's addition stays everything.

### Amended: the hubs' regions, tried and dropped (2026-11-17)

A function most tests reach selects them all wherever a hunk lies in it,
so the recording kept, beside the map, each hub's code regions that some
tests ran and others not (29c746696): of each function 90% or more of the
tests reach, each region 1% to 99% of them ran, and each test's bits over
those, for a hunk in such a region to select only the tests that ran it.

The first recording (test-map run 43990610976, of 7fe40bde3): 5,027 hubs,
79,135 code regions, 30,262 varying kept; the map 21.8 MB (7.2 MB gzipped),
the regions 28.0 MB (7.5 MB gzipped). Its parts took 7,311 s summed where
the recording before took 4,014 (+82%), its longest part 1,197 s where it
took 729, and the artifact every push downloads 15.4 MB where it was 7.5
(7.3 s here). The interaction sanity's three examples, placed in that
map by their changed text, selected the same with the regions as without:
every hub hunk of theirs, `lower_crate`'s among them, lies in a region
every test runs, which narrows nothing. Their changes touch code every
compile runs. The recording side is reverted.

What the examples did show: 2905d5cdc's own change, src/output.rs, was
291 files by the stale map's fallback ("by every test of its functions",
a file the map's commit and the change's base differ on) and 7 by a map
that places it; which the selector's placing a moved line by its text
addresses (the next amendment).

### Amended: a moved line is placed by its text, and the fallback runs what runs on every change (2026-11-17)

A side of a push's selection numbers its hunks by its base, `from`, not
the map's commit; where a src file differs between the two, its hunks
weren't the map's lines, so every test the map saw run any of the file's
functions ran (`sideSelection`'s fallback). A hunk is now placed at the
map's line its text has, where neither the map's commit nor `from`
changed that text since (`mapLine`, `hunksAtMap`, as git places a moved
line), and `pick` reads it as any other; only a hunk in text that changed
since the map falls back. And the fallback runs, as `pick` does, each test
the map says runs on every change (`movedUnits`), which it ran none of.

Of the 24 commits to src since the map at 7150cc3ad, by that map, each
commit's own side: 2905d5cdc stays the fallback, its src/output.rs hunks
being in text the #1681 series changed since (290 files to 362, the
always-run tests added); d0ce7d1ab is placed, in `lower_crate`, every
test's (285 to 366, the fallback having run 81 fewer than the placed
change); 8213a7d4f is placed outside any function, everything (293 to
all); 6fb53cf24 365 to 348 files, 86b8aa91f 248 files and 2,038 cases to
204 and 1,410. The 7 of 2905d5cdc's own side was a newer map's, at
7fe40bde3, which has its text. So fresher maps, not placement, are what
narrows a change in code under active edit.

### Amended: a recording carries what the commits since its map can't change (2026-11-17)

A map trails `main` by its recording, 22 to 44 minutes on CI's 8 machines,
some 4,000 s of their parts, and a push read one a median 6 commits behind
(at most 11, of ioCM's 84 pushes' selections of the day): a change near
code another's commit changed since falls back to every test of its file's
functions. So a recording is incremental where an earlier map serves
(`planCarry`): it records the tests the selector's full rule selects of
the commits since that map, `fullSelection`, the very selection CI's check
of `main` makes (`selectChange`'s `full`), never the push rule, so the two
can't diverge, with what that rule counts as a change, the runtime, std,
fixtures and each file a test reads, as well as src; each test the map
has none of or ran on every change; and carries every other test's
functions over from that map by each function's symbol (`remapBits`), its
index and lines having moved. A test that ran none of a changed function
runs the same code, so the same functions (why a skip is sound, above). A
function new since is in no carried test, as only changed code calls it.

The map now keeps its functions' symbols (`names`), the full recording
its chain started from (`base`), and what it carried (`carried`). A
recording is full where the selector's rule says everything, where no
earlier map has names, where the nearest is past `carryCommits` (40), where
its chain's full recording is past `baseDays` (1) or `baseCommits` (200),
so a drift can't compound between nightlies, and where the nightly's drift
check failed (`driftFlag`, kept in the map store until a nightly's
passes). The nightly records HEAD's map whole (`full`) and checks it
against what a carry from the nearest earlier map with names makes of it
(`driftCheck`): a test whose functions differ fails the run, says which,
and flags the store.

Proof: test/test-map.test.ts's merge of a recording in parts, where an
incremental recording of the same tests merges to the same map as the
whole one, and "a test carried over keeps its functions by their
symbols"; mutations `carry-new-function-counted`,
`carry-skips-what-ran-always` and `carry-case-file-own-lost`.

