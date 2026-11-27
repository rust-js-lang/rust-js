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
| `std/**`, `crates/rust-js-program/src/runtime/**` | its package tests, every case, every file the map saw compile (not those that ran no compiler) |
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
| A line in a function of `crates/rust-js-lower/src/std_types/slice.rs` | 353 s | 214 s: 94 of 356 files, 71 of 2,649 cases; the nets its paths run are most of it (the twin net 101 s, the crate corpus 69 s) |
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
pass took longer than the check: katniss agent's took 594 s and 516 s after a
491 s check, and worker-02 agent's 1,228 s. Her push gave up after main moved
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
corpus cases alone, `capped corpus of the interaction`. worker-02 agent's runtime
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
where they most meet: worker-03 agent's tidy refactor's cap ran nothing. Others'
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
(`whoRuns`); without either no line is written, saying which isn't set
(#2322), where it was the host's name, apart from the machine's other
runs. It's written on every exit,
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

worker-02 agent's test-only push selected everything: "crates/rust-js-program/src/js/writes.rs's change
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
outcome can turn on: worker-02 agent's heap split's `mod derived;` ("writes an
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

katniss agent's adapter-family flips each grew `HELD_FLIPPED: [&str; N]` by one
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
4ba38fa64's crates/rust-js-program/src/prepare/import_names.rs did on every rebase's incoming
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
functions every test reaches (`lower_crate`, crates/rust-js-emit/src/output.rs's), which the
hubs' region coverage is to narrow.

### Amended: a name no item declared, added to a `use`, is no change of its own (2026-11-17)

4ba38fa64 selected everything for crates/rust-js-program/src/js.rs:43, `imports::{Import,
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

What the examples did show: 2905d5cdc's own change, crates/rust-js-emit/src/output.rs, was
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
commit's own side: 2905d5cdc stays the fallback, its crates/rust-js-emit/src/output.rs hunks
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


### Amended: a private `use` changed, and a function's signature, select what they can change (2026-11-18)

A hunk outside any function's code selected everything, with every net and
the crate corpus: 7963110df and 191ad93e9 for `use crate::js::{..,
Name, ..};` at assigned_breaks.rs:5, then `fn function_names(e: &Expr,
name: &Name)` at stepped_whiles.rs:96, its parameter's type changed. Two
such hunks are now said:

- **A private `use`** (`useChanged`), its names added, taken out or moved,
  no glob, no `pub`: it changes how code in its module's scope resolves a
  name, a path's or a method's by a trait it brings, so it selects every
  function of the file and of the modules in it (`src/a/b.rs` and
  `src/a/b/**`; the crate's of main.rs), not only those that name its
  words, as a trait resolves a method no line names. Only a function calls
  a trait's method: a constant's or a static's value is const, which
  calls none. Where a word of the `use` is named in that scope outside any
  function's code and its signature, a type's field, an `impl`'s header,
  a constant's value or an item a macro makes, everything still. A glob's
  names no word says, and a `pub` one is read wherever it's imported:
  everything.
- **A function's signature** (`signatureChanged`), with new items written
  before it that nothing named before (`unreferencedOnly`): its body and
  each function that names it, a call or a path, the module's of a private
  one, the crate's of a `pub` one; one named outside any function's code,
  everything. Of sources it can't read, everything.

Measured on the select-all pushes of 2026-11-17 replayed against the map
nearest each: 7963110df and 191ad93e9 select some, 371 of 374 files and
2,658 of 2,731 cases (the change touches preparation, which every compile
runs), but no longer every net and the crate corpus; 0eba652be (an
attribute, `#[cfg(test)]`, on three `impl`s) and a628bf769 (a field added
to a struct) are everything still, as is ec5f73747 (a workflow). Test:
test/test-map.test.ts, "a private `use` changed is each function of its
module's" and "a function's signature changed is its body's change";
mutations `use-change-glob-taken`, `use-change-only-namers`,
`use-change-type-field-unseen`, `signature-change-callers-unseen` and
`signature-change-pub-module-only`, each caught.

### Amended: a sanity pass runs what fits a minute, what reaches the change first (2026-11-18)

Of the 141 pushes in the Mac mini's log, 27 needed a second round or a
third; past their first, they spent 5,905 s, 3,670 s of it in sanity
passes. Each since 2026-11-16 was capped: past 40 files, the
interaction's every corpus case (15 rounds, 41–248 s); where both sides
say everything, the change's own selection by the map, the whole suite
again for a compiler change (6 rounds, 220–228 s). One of 16 files took
229 s: a file's cost isn't its count. A longer pass is also a moving
`main`'s next round.

Now every sanity pass, the interaction or the cap, runs what fits
`sanitySeconds`, 45 s of the machine's cores past check:fast's own start
of some 25 s (`budgeted`). Each test file and corpus case is ranked by how
many of the change's own functions it ran as the map recorded them
(`reachOfFunctions`): the functions its diff changes (`pick`'s), or of a
change the map can't place, every function of its compiler files
(`functionsOf`); the cheaper first between equals, then the selection's
order. A file costs its usual as check:fast's jobs keep it, a case the
corpus's whole run over its cases (`costsOf`); one not yet measured 10 s,
0.3 s. Each is taken where it fits what's left, none that alone is longer
than the budget, as `test/metamorphic.test.ts`'s 180 s. The pass says
`capped <n> of <m> files and <k> of <l> cases in 45 s, by reach of the
change's <f> functions` (or `cheapest first` where it has none).

The trade is a local minute against an interaction reaching `main` and
turning it red, a CI round and a fix. So what each pass ran and left is
kept by the commit it pushed (`recordSanity`, the main checkout's
`.cache/sanity-runs/`, 16 MB, `bun run disk`'s). Where `main` goes red
from two pushes meeting, the failing test is in its pass's `ran` or its
`skipped`; one that the budget skipped more than rarely is the cue to
raise it (docs/dev-loop.md). The old `sanityBudget`'s corpus-only form and
check:fast's own cap are gone: `selectChange` owns the pass whole.

Tests: test/select.test.ts, "a sanity pass runs what reaches most of the
change first, within its budget"; mutations `sanity-budget-unranked`, `sanity-budget-dearer-first`,
`sanity-budget-overrun`, `sanity-skipped-unrecorded`.

### Amended: a known list's conflict on rebase is merged as a set (2026-11-18)

Two pushes that each took a line off test/metamorphic-known.txt, worker-02 agent's
#2054 and worker-03 agent's #2056, conflicted where the lines were neighbours, and
`bun run push` stopped the second, as it stops every conflict it can't
resolve mechanically. A known list is a set of lines, each standing for
itself in a block under its issue: whatever either side took off is gone
from the merge, and whatever either added is in it. So a rebase's conflict
in a line set (`lineSet` in scripts/push.ts: each `test/*-known.txt` and
rustc's three lists) is merged as one: of git's own merge with its base
(`git merge-file --diff3` of the rebase's three stages), each conflict's
lines are counted, a line's count is ours' and theirs' less the base's,
at most the larger side's, in ours' order with theirs' additions after the
line before them in theirs (`mergedLineSet`). Where each side both added
and took off lines in one conflict, a line each side changed (an id list of
test/iterator-matrix-known.txt), no set says which is the list's, and the
push still stops. The blessed baselines of totals, docs/readability.txt and
the coverage lists, aren't line sets: they're made again by their bless, as
before. One rule, `remakeable`, says which conflicts a push resolves.

The tests of each merge moved with this to test/push.test.ts, "a rebase's
conflict in a line set keeps what neither side took off and what either
added, a line both changed stops"; mutations
`line-set-conflict-stops-the-push`, `line-set-keeps-what-a-side-took-off`,
`line-set-merges-a-line-both-changed`.

## Amendment: a fast phase refuses first what would refuse a push later (2026-11-19)

Two refusals came late. Push-guard's, that a push undoes a commit on
`main`, ran only as git's pre-push hook, after the whole check: 4 to 8
minutes on the Mac mini, spent each time on #2146's, #2139's and one
before, whose known-list rows it took for an undoing. And the mutation
catalog's tests that each mutation's `find` applies once and that its
test pattern names a test of its file ran minutes into check:fast: a
retarget or a test moved by concept failed there. Now `bun run push`'s
second phase, `fast`, right after the rebase, runs push-guard's `undone`,
the one rule the hook runs too, and those two tests (`FAST_TESTS`),
about a second together, before the build. Since d4c76fa8b's first push,
refused minutes in by another of that file's tests, a catalog naming a
file not its own, `FAST_TESTS` is the file whole, about 4 s. And rows a push takes off a
`test/*-known.txt`, each under the heading of an issue one of its commits
`Fixes`, are that issue fixed, not an undoing of the commit that listed
them (`fixedRowsOnly`, `fixedIssues`); rows under another issue's
heading, a `#difference`'s, or a row added still are. A snapshot of a new
corpus case and the readability baseline stay the corpus run's: which
cases snapshot is the harness's to say, and the baseline needs the
corpus compiled. Held by test/push.test.ts's "a push's fast phase runs
first" and test/push-guard.test.ts's "rows taken off a known list";
mutations `push-fast-unguarded`, `push-fast-tests-after-build`,
`push-guard-fixed-rows-undo`, `push-guard-fixed-rows-any-heading` and
`push-guard-first-fixed-issue`.

## Amendment (2026-11-19): rows taken off under an issue the push `Refs`

push-guard read rows a push takes off a `test/*-known.txt` as that
issue fixed only under the heading of an issue it `Fixes`. A partial fix
of an issue of many rows, #1892's one neighbours row of four, which the
issue keeps open for the rest, cites it `Refs`, and was refused as undoing
the commit that listed the row. Now `fixedIssues` reads the issues a push
names, `Fixes` or `Refs`: naming the issue is what makes taking its rows
off deliberate. Rows under an issue no commit names, under a
`#difference`, or a row added still undo the commit that listed them.
test/push-guard.test.ts: a `Refs` row comes off, one under an unnamed
issue is still refused. Mutation `push-guard-refs-rows-undo`.

## Amendment (2026-11-20, #2162): a visibility change of a name declared once

Change E's sweep narrows answers' visibility in many files. Before this, each narrowing's signature hunk was "that function's change", so a push selected nearly every test (396 files, 2,805 cases, 400–630 s).

**A signature hunk whose change is only its visibility, or its comments, selects nothing where nothing else can resolve in its place.** Three conditions:
1. The function's name is declared once in the sources at the map's commit (`fn`, a trait's or an impl's method, `const`, `static`, a type, `mod`, a macro).
2. No line the change adds declares it.
3. It isn't the prelude's `drop`, which a glob's call falls back to.

Of a method, `self` among its parameters, there's a fourth condition. rustc's method probe passes over an inherent method a caller can't see and takes a trait's method of that name, std's or a dependency's too: a private `len` beside a trait's `len` gives the trait's, as probed. So a method selects nothing only where no code beyond the crate names it: the sysroot's crates' metadata, std's sources and the sources of each package Cargo resolves (`namedBeyondTheCrate`, a single pass, about 1.7 s). Otherwise the hunk is that function's change, as before.

If it compiles, each use resolves to the item it did before, so what the compiler does is the same. The select line says how many hunks were selected nothing this way. Held by test/test-map.test.ts's "a visibility change of a name declared once selects nothing" and four mutations (scripts/mutations/test-map.ts).

## Amendment (2026-11-20): a round past a rebase spends only what the commits it brought can change

A push whose rebase brought commits ran its round again. 69240fd35's second round spent fast 21 s, the mutants type-check 87 s and a sanity pass 118 s. worker-03 agent's S4 push went past 900 s this way.

**The round now spends only what those commits can change:**
- The mutants are type-checked again only where the incoming commits touch a Rust source, a mutation list or the toolchain, any of which a `replace` may read (`mutantsAgain`). Otherwise each `replace` type-checks as it did the round before.
- The sanity pass checks nothing where the selector's interaction of the change and the incoming commits is none (`sanitySkipped`). No test is reached by both, so each one's outcome is the round before's or the incoming pushers'.

The push's line then says "none (incoming reach nothing this change reaches)", so a later surprise can be traced to a skipped round. `main`'s CI check runs whole after the push either way. Held by test/push.test.ts's "a round past a rebase type-checks the mutants and checks the interaction only where the incoming commits can change them", and the mutations `push-mutants-every-round` and `push-sanity-never-skipped`.

## Amended: the item layer, in shadow (#2288)

Half of the compiler's pushes selected everything (9 of 18 on the Mac
mini; 13 of the 20 compiler commits replayed here): a line outside any
function's counted code, an enum's variant, a const, an `impl`'s header, a
`use`, or a new file the map can't say anything of. The map now records an
item layer at its commit (scripts/testMapItems.ts), from rust-analyzer's
SCIP index of the crate (the pinned toolchain's component; 38–46 s, 4 GB):
each item outside a function, its extent, and the functions that reach it.
They are SCIP's references to it or a member, every function whose counted
code names it or a member, its variants', fields' and methods' identifiers,
and, through another item that names it (`passes: JUMPS` in a const
`STAGES` a function iterates), that one's, to a fixed point. Each `impl`'s
header reaches its methods and every function that references its trait or
a method of it; each function's own name finds one moved to a new file;
`#[cfg(test)]` modules are known as code no test of the map runs. Where the
index can't be made, the map has no layer, said, and selects as before;
never part of one.

The rustc crates resolve to nothing in that index (0 references; neither
`rustc.source` nor the CLI's config reaches them), and SCIP marks nothing
unresolved: 76% of the map's 23,943 functions hold an identifier it leaves
unresolved, measured by position. A fallback that took each such function
for one that reaches every item its file imports would have selected
everything again, so it isn't used; the names carry what SCIP misses, and
the mutation catalog holds the result to account.

What a change outside any function selects with the layer: the outermost
item holding the line's reachers, an `impl` header's, or, between an
inherent `impl`'s methods, the functions that name a method it adds (an
added method can only shadow a trait's of its name there, where a trait
impl's overrides a default); a `use`'s, through the names it brings in, each
use of them; a const's or a static's value, its holders and reachers; a doc
comment or an attribute, nothing; a signature line, its function and the
functions that name it; a new file, the map's functions of the names it
defines and what reaches the items it defines, or everything where it
defines a macro.

The evidence. Of the mutation catalog's 54 mutations of `src/` that select
everything today, 50 select with the layer, each holding the test that
catches the mutation, and 4 still select everything (the "writes an item"
text test taking a JS `const` in a Rust string, `union(` for a union).
Replayed against each commit's nearest map, 13 of 20 compiler commits that
select everything select 2 with the layer; over 389 commits against one
map, 42 select 12, those of files outside the compiler. The honest finding:
most changes outside a function touch central code, an enum every lowering
matches, a function every compile runs, tidy's `STAGES`, whose selection
really is nearly everything (8 of the 11 that turned picked held 3,271 to
3,279 of 3,279 tests and cases; 3 held 108 to 229). There the next lever is
the full check's own speed, not selection.

It runs in shadow first: each push's and check:fast's selection is the
map's without the layer, as before, and the layer's beside it is said on
the push's line, `[items: <its outcome>, missed <n>]`, `missed` each test
that failed in the run the layer's would have left out (`shadowMissed`).
The switch comes after some 30 pushes across the machines with none
missed, read from the machines' logs.

## Amended: a round's React port runs the round's own compiler (#2317, 2026-11-22)

A push's React regen ran `target/debug/rust-js` beside the check. Something
in the round relinked that file while the port's `cargo check` was running
it, and the push failed with "could not execute target/debug/rust-js (No such file)" after
its whole check had passed. The relink came from the std phase:
`std-fresh.ts` wrote std/compiled-std.json again with the same bytes, and
build.rs tracks that file by its time, so the next `cargo build`, the
regen's or check:fast's, rebuilt the compiler. The environment wasn't the
cause.

Two changes fix it. First, the compiler writes its reports through
emit's `publish_file`, which leaves a file holding the same bytes as it
is, so nothing in a round makes a build relink. Second, the regen gets the
round's own link of the compiler its build phase built (`portRegen`), so
even a relink can't reach it. scripts/own-compiler.ts is the one owner of a
run's own compiler. check:fast's jobs, a push round's regen and the mutation
runner's checkout build are links it makes in `target/<kind>/<pid>/`. A run
gone without exiting leaves its folder, which the next run of that kind
removes. test/push.test.ts holds this: a relink during a regen leaves the
regen's compiler as it was, and no other script links the checkout's build.

## Amendment: what a rebase resolved as main's is made again in push run again (2026-11-24, #2417)

A rebase that changes what push has loaded runs push again from the
rebased tree (#2275). That run's own rebase finds nothing left to
resolve, so the files the first rebase resolved as main's, std/'s
generated files and blessed baselines, weren't made again: worker-03 agent's push
over 691b08b57 kept main's `std/runtime.js`, without the `$provided`
helper her `std/iter.js` imports, and only test/mutations.test.ts caught
it. `againEnv` now carries them, `RUST_JS_PUSH_REMADE`, and the run
again's first round makes them again with what its own rebase resolved.
test/push.test.ts' "push run again makes again what its rebase resolved
as main's" holds it; mutation `remade-uncarried-into-the-run-again`.

## Amendment (2026-11-24, #2288): a line outside any function, by its kind, then its item's name

worker-02 agent's count of main's checks since 2026-11-21: 130 of 188 selected
everything, and a Rust line outside any function's counted code was 71%
of the excess, about 1,930 runner-minutes in 3 days. Of 41 such changes
located: an item's definition 27 (a variant, a field, a `const`'s value,
an `impl` or type header), `use`/`mod` lines 11, `cfg(test)` code 3, and
comments 2. Tu chose a heuristic, with the nightly's audit as its
backstop. Each line not held by a counted function, and not answered by
the item layer, is now read by its kind:

- **A `mod x;` or `extern crate`:** nothing of its own. What it brings in
  changes only where a function using it changed too, and a new module's
  file is new since the map. A `use` keeps `useChanged`'s and
  `useNamed`'s answers, as a `use` may change what a type's field names
  with no function changed.
- **Code of a `#[cfg(test)]` module:** no test file. `cargo test`, which
  check:fast runs on any change of the compiler's Rust, is its test.
- **Any other line of an item:** the item it writes or is in (its own
  header, else the nearest above it indented less), and the counted
  functions whose source names it as a word.
- **Everything, the reason naming it,** where the name is shorter than 3
  letters, more than 60% of its crate's counted functions name it, it's
  named by another item (a constant's or static's initializer, a type's
  field outside any function), or no item is found.

Misses it accepts, the audit's: a use through `Self`, a glob import's
alias, a macro's expansion, a method reached by trait resolution. Tests:
test-map.test.ts' rows (a variant selects its enum's namers; a short
name and a common one select everything; a constant's type changed
selects its namers). Mutations: `test-map-item-selects-everything`,
`-short-name-searched`, `-common-name-searched`,
`-item-named-by-another-unseen`.

Amended again (2026-11-24, the lead's and worker-02 agent's probe of the rule,
#2288 comment 7127339825): what changes what code does with no function
changed.

- **A new inherent `impl`'s methods,** each by its name, as a method of
  an inherent `impl` goes before a trait's of its name: a new `len`
  changes what each `x.len()` calls.
- **A new free item** (`fn`, `const`, `struct`, `type`) that some
  function names selects its namers, as it may shadow a glob import's
  item of its name, or make a `match` arm's binding a constant's pattern;
  one nothing names is still nothing.
- **A `macro_rules!` line:** the macro's namers, besides its own line's
  item's, as every expansion changes.
- **An outer attribute's line alone** (`#[cfg]`, `#[derive]`, `#[serde]`):
  the item below it, and of a `mod x;` below it, that module's functions.
- **A new `impl Tr for X`:** what names `X`, and what names `Tr`, as code
  bound by `Tr` reaches the new impl without naming `X`.
- **A `mod x;` beside `use`s** in one hunk: the `use`s' answer
  (`useChanged`, `useNamed`), the `mod` lines set aside.

Each holds by a row of test-map.test.ts and a mutation: `test-map-inherent-methods-unsearched`,
`-new-item-unsearched`, `-macro-body-unseen`, `-attribute-above-unread`,
`-trait-impl-trait-unsearched`, `-mod-beside-use-everything`. Left as they are:
variants reordered and a default type parameter, which still select
everything (over-selecting).

## Amendment (2026-11-24, #2484): a rebase's commits compiled first, and a round carried on past a mechanical merge

worker-02 agent's design, from two of the day's costs.

**A. A compile gate right after a rebase that brought commits** (8fdf5d8ae).
A semantic conflict, the change removing or renaming what an incoming
commit newly calls, merges cleanly, and the sanity pass runs no lint, so
it was found only by a full check or CI: 567 s late on the Mac mini.
`compileGate` runs `bun run typecheck` where both the change and the
incoming commits touch TS or JS, and `cargo clippy --all-targets` where
both touch Rust, check:fast's own lint commands, whose caches the next
check reuses. One side alone was compiled by its own push, so only both
can bring a new conflict. Its time is the line's `gate` phase.

**B. A round carried on by what the push wrote, not by what the rebase
rewrote.** Push run again from a rebased tree (#2286) carried on from the
round before only where the branch's commits were patch for patch those it
checked. A rebase that resolved a conflict mechanically rewrote a patch by
itself: std/'s generated files taken as main's and made again, a blessed
baseline, a ratchet's lower count, an ADR's two amendments, a line set,
TESTS.md's `Changed on` lines. Each turned the run again into a full check
where a round in one process runs the sanity pass. Now:
- `roundRefused` compares the patches without those files (`withoutRemade`,
  by `remakeable`, the one rule of what a rebase resolves mechanically).
  They are push's own writes, each made and checked again by its own phase
  (std/ by `std-fresh`) or by the tests that read it, which the sanity
  pass picks as the incoming commits touch it. Any other file's difference
  still refuses.
- A refusal for the patches runs that sanity pass, `--sanity=<the commit
  checked>`, with nothing of the round reused: its fast tests, mutants and
  React gate run again (`carryOn`). The refused round's work is lost, not
  its narrowing. A full check is left for a round whose base isn't an
  ancestor, or a tree with a change.
- Each round past the first says on the line how it came by the one
  before: `sanity carried …`, or `… refused (<why>) …` (`roundSaid`).

The files B leaves out include those `remadeStd` and `regenerate` write
after the rebase and amend into the top commit: push's own mechanical
writes, never the change's, so neither refuses a carried round.

Held by test/push.test.ts: "a rebase that brought commits is compiled
before its tests, where both sides touch the language" (A), and "push run
again carries on past a rebase's mechanical merge, and a refused carry is
a sanity pass" (B). Mutations `rebase-gate-never-run`,
`rebase-gate-own-side-only`, `push-round-remade-compared`,
`push-round-refused-whole`, `push-round-refused-reused` and
`push-round-carry-unsaid`.

## Amendment (2026-11-24, #2523): an item another item names is searched through it

#2288's rule maps a line outside any function to the functions that name
its item, a variant's line to its enum's namers. But where another item
named it, a struct's field's type, a function's signature, a constant's
initializer, the rule stopped: `named by another item`, everything. #890's
naming relays each added a `Role` variant, and `fn declared(role: Role)`'s
signature named `Role`, so each ran a whole check (665 and 883 s on the
M1).

Now a site of the name in another item is followed:

- a function's signature: the function it opens, which compiles its
  parameter;
- another item, a field's struct, an alias, a constant whose initializer
  names it: what names that item, searched in turn, each item once.

A site whose item can't be named, a macro's invocation outside any
function, is still everything. The short-name and common-name valves
hold at each step. A line of a `use` written over several lines is a
`use`'s, which reads nothing; and a name added to a `use` whose list
rustfmt reflowed over more lines or fewer is the rule's for a name added
to a `use` in place, its lines each of paths and names alone, no item's
keyword (177cf3074's `Copied` ran its own push whole).

Mutations `test-map-item-named-by-another-unseen` (rewritten),
`test-map-signature-namer-unopened`, `test-map-other-item-everything`,
`test-map-use-reflow-everything`, `test-map-use-list-takes-items`.

## Amendment (2026-11-25, #2584, #2575, #2587): a `const fn` isn't a constant; a net's paths name a module whole; TESTS.md merged by git

Three gaps of the selection and of the merges around it:

- **A `const fn`'s signature read as a constant named `fn`.** The search
  through another item (#2523) took a site's line for a constant's
  initializer by `const <name>` alone, so `pub(crate) const fn first(of:
  &First)` was a constant `fn`, too short a name to search, and
  everything. #2579's fix hoisting `struct First` in reads.rs selected
  everything on two pushes, 1,041 s each on the Mac mini. A constant's or
  a static's line is `constItem`'s alone now, the name its type's `:`
  follows, which the value rule's reader shares; a `const fn`'s line is a
  signature, the function it opens. Test: "a moved struct named by a
  const fn's signature selects the function it opens"; mutation
  `test-map-const-fn-a-constant`.
- **A net's paths named half a module.** The drops' paths
  (scripts/iteratorSlice.ts' `dropPaths`, which the consumer net's and the
  async table's paths hold) were `drops/**` without `drops.rs`, so
  845c1a3e9's change to `check_drops_given` there ran neither net, and a
  later push that selected everything found the consumer net's known list
  stale (#2575). They're each module whole now, its file and its folder,
  `analysis/unwinding/**` too; test/ci.test.ts' "each net's paths take a
  module's own file with its folder" holds every catalog of the slices to
  it. Mutation `drop-paths-folder-alone`.
- **TESTS.md's `Changed on` lines conflicted on nearly every relay.**
  `bun run push` merged its own rebases' (`mergedChanges`); a
  cherry-pick or a `git am -3` stopped, a minute by hand each. The rule
  is scripts/merge-changes.ts', git's merge driver `changes-log`
  (.gitattributes' `TESTS.md merge=changes-log`, named in git's config by
  `bun install`'s `prepare`), so git merges them in every merge it makes,
  and push resolves by the same rule a conflict git left it where the
  driver isn't named. TESTS.md stays out of a round's patches as before.
  Test: "a cherry-pick's conflict in TESTS.md's `Changed on` lines is
  merged by git's driver; one in its table stops it"; mutation
  `changes-log-driver-merges-nothing`.

## Amendment (2026-11-27, #2608): an item's namers are where its crate and module path let it be named

The search for an item's namers (#2288, #2523) took every line of every
crate of the compiler that had its bare name as a word. A same-named item
of a crate that can't see it, or a word of a JS text there, counted:
dedb8fc2fe's closures.rs change, outside any function, was searched
through `ClosureFacts`'s namers to rust-js-lower's `named`, then to
rust-js-emit's prettier.rs, which doesn't depend on rust-js-lower, and to
a `configs` of its Prettier script, outside any function at
prettier.rs:47: everything.

Now `pick`'s `seenFrom` answers where the item `name` that a file
declares can be named, by its declaration there, and the three searches
that read an item's namers ask it: the item rule's (`byOneName`), a
constant's value's (`constValue`) and a signature's (`signatureChanged`,
whose own `pub`-or-module test it replaces):

- a private item, free or of an inherent `impl`: its module's files,
  `moduleOf`'s;
- `pub(crate)`, `pub(super)` or `pub(in ..)`: its crate's;
- `pub`, a macro, or an item a macro's body writes wherever it's invoked:
  each crate whose build holds its crate (scripts/compiler-crates.ts'
  `cratesBuilding`, Cargo.toml's dependencies, the one authority on
  which crate depends on which), a re-export's importers among them;
- an item of a trait, or of a trait's `impl`, which has no `pub` of its
  own and which code bound by the trait calls: wherever the trait is;
- a name the file doesn't declare, an `impl`'s type or trait declared
  elsewhere, or an item a change is writing: anywhere, as before.

The rule narrows only what's searched; its valves, a name too short or
one most of a crate's functions name, hold as before. Replayed over the
16 compiler commits since the map at ac9716c758, the running selection
(the item layer in shadow) is the same: 14 everything, 2 picks of 147 and
125 files unchanged. The chase from `ClosureFacts` walks most of the
compiler's types, and now ends at another dead end, rust-js-mir's
`StdLowering` at dispatch/known_calls.rs:15, a multi-line `type` alias's
last line at column 0, which no item holds; the item layer's shadow
selects 454 of the 457 test files and every corpus case for
dedb8fc2fe's change, so its readers are nearly every test. Such a chase takes about 3.6 s where it took 0.6 s,
reaching further before its dead end.

Test: "an item's namers are those its crate and module path let name it,
not another crate's word of its name". Mutations
`test-map-namers-any-crate`, `test-map-namers-own-crate-only`,
`test-map-trait-item-by-own-visibility`; `signature-change-pub-module-only`
retargeted to the shared rule.

## Amendment (2026-11-27, #2608): every line of a multi-line item is its item's

The previous amendment's chase from dedb8fc2fe's closures.rs line 246 ended
at rust-js-mir's `StdLowering`, named at dispatch/known_calls.rs:15 by
`) -> Option<std_calls::StdLowering<'c, 'a, 'tcx>>;`. That is the last line
of the `type StdHandler<..> = for<'s, 'c> fn(` alias, at column 0. `itemOf`
took a line's item to be the nearest header indented less than the line,
and nothing is indented less than column 0. So the line was no item's, and
the search ended in everything. The same walk read past a line at column 0
that is no item's header: a line inside an item macro's invocation,
`fields! {`, was taken for the item above the invocation, whose namers it
selected.

Now `itemOf` gives a line that goes on with one above it at its indent its
opener's item, as rustfmt lays out what spans lines. Such a line (`goesOn`)
is a `where`, a lone `{`, or one that starts by closing a bracket: `) ->`,
`>;`, `];`, `}`. That covers a type alias's, a const's and a static's last
line, a struct's or an enum's closing brace past a `where`, and a function's.
A block no item opens, an `if`'s, is the item it's in. A line inside a block
opened at column 0 by no item's header, an item macro's invocation, is no
item's: its item isn't found, and so everything, as nothing says which
items the invocation writes.

Replayed over the 16 compiler commits since the map at ac9716c758, without
the item layer, the running selection is the same: 14 everything, 2 picks,
none dropping a file. The dead end moved again. For line 246, for #2623's
own_steps.rs line 176 (590bf56b8) and for three more commits it is now `Cx`,
too short a name to search, reached after a longer chase: 6.9–8.2 s, and
9.4 s for #2623. Earlier chases took 3.5–4.5 s, and before the previous
amendment 0.4–0.7 s.

That is the third dead end in a row of one mechanism, the search for an
item's namers by its name as text: a crate that can't see it, a line no item
held, and now a name too short. ADR 1085's cluster rule applies, so the
short-name valve is not patched. #2608's design comment proposes that the
map record the references rustc resolves in the compiler's crates, from the
counting build, and that `pick` walk them in place of the search.
#2626 is the stopgap: without the item layer, a comment line in a mixed
hunk is searched at all.

Test: "a multi-line item's last line at column 0 is its item's, a type
alias's as known_calls.rs:15's; a macro invocation's lines, no item's
above". Mutations `test-map-item-last-line-unowned` and
`test-map-macro-invocation-item-above`.

## Amended (2026-11-27, #2482): a change's dependants, ADR 1209

[1209](1209-a-changes-dependants-are-what-the-map-recorded-reaching-it.md)
records with this map what else reaches each test, and asks one lookup of
it (scripts/dependants.ts). There are two sources:

- each test file's loads, its children's too (#2482). At its switch these
  replace the folder row of the 2026-11-16 table; until then they run in
  shadow;
- rustc's resolved references (#2608). At its switch these replace the
  name search for a line outside any function's counted code.

The map is v5, with `paths` and `loads`; `defs` and `edges` are reserved.
`Selection.shadow` is `Selection.shadows`, the item layer's and the
loads' each said on the push's line.
