# 1093. The check before a push is the push's check, held to it by a test

Status: Accepted (the user's decision, 2026-11-05, #732). Extends
[0104](0104-parallel-tests.md) and [1030](1030-what-only-the-nightly-runs.md).
Amended by [1101](1101-the-inner-check-runs-what-a-change-reaches.md)
(#877): plain `check:fast`, the inner loop's, runs what a change reaches
by the test map; `--all`, this check, is unchanged.

Case: N ([0262](0262-when-rust-and-js-disagree.md)). This changes only how
rust-js is checked. It changes no output.

## Context

`bun run check:fast --all` was the check run before a push. A push's
check, `check.yml`, ran more than it did, and the extra suites went red
on `main` unseen:

- **`cargo test --locked`**, the compiler's unit tests, ran in `check.yml`'s
  `lint` job alone. `finish_is_the_last_stage_in_its_order` failed after
  801d40c55 (#523) and was fixed in ce09afc6c.
- **The browsers' files** (`scripts/shard.ts`'s `browsers`) were left to
  CI, as beside the corpus's compiles they starve. These files also hold
  tests that need no browser. "the pilot's committed JS" failed at
  336984bff (#673) and was blessed again in 5c80c52c5.

Agents had to remember to run both by hand, which made a second list of
what to run before a push. AGENTS.md, DEVELOPMENT.md and each brief kept
their own copy of that list.

## Decision

**`check:fast --all` runs what a push's check runs that can run on the
Mac.** The check before a push is `check:fast --all` and the change's
mutations, with no other list.

- **`cargo test --locked`** is a job that runs with the others. Its
  timeout is 600 s, as its build waits on clippy's lock of `target/`.
- **The Vite example's build**, `check.yml`'s `examples` job, is a job
  too. It takes 4 s.
- **The browsers' files run after the other jobs, on their own**, as
  `scripts/test.ts` runs them: 53 s on the M3 Max. So every file of the
  push's suite runs.
- **Each job has its own timeout** (`Job.timeout`, 290 s unless the job
  sets one).

**`test/ci.test.ts` holds the two lists together.** `scripts/checkFast.ts`
exports its jobs (`jobs`) and what it leaves to CI (`leftToCi`), each with
its reason. Two tests check them:

- **"each command a push's check runs is check:fast --all's, or left to
  CI by name".** Each `run:` of `check.yml`'s jobs, and each reusable
  workflow a job `uses:`, must start with a job's `ci` command or a
  `leftToCi` entry. Each `leftToCi` entry must still match something in
  `check.yml`.
- **"each test file a push's suite runs, check:fast --all runs".** Each
  `test/*.test.ts` file that is not the nightly's must be run by a `bun
  test` job of `--all`, by its filters and its ignore patterns.

What `--all` leaves to CI, and why: the WASM compiler (a build of about
half an hour) and the two tests it runs with it; rustc's whole suite on 6
machines and the mutations of all of a check's changes (on the Mac,
`bun run test:rustc <dirs>` and the mutations the change adds); a
machine's setup; the run's own planning; and the bless patch of a red run.

## Consequences

- A new step in `check.yml`, or a new test file the push's suite runs,
  fails `test/ci.test.ts` until `check:fast --all` runs it, or `leftToCi`
  says why it can't.
- Both earlier failures fail `check:fast --all` when they are put back:
  `cargo test` fails on `finish_is_the_last_stage_in_its_order`, and the
  browsers' job fails on "the pilot's committed JS" (2026-11-05).
- `--all` takes about a minute longer than before: the browsers' 53 s
  after the rest, and `cargo test` and the example beside them.
- The check of a change, without `--all`, is as before.

## Rejected

- **Moving the pilot's committed-JS test out of its browser file.**
  `next.test.ts`, `vite.test.ts` and `webapi-events.test.ts` also hold
  tests that need no browser. Splitting each file would leave a list of
  which tests need a browser, and that list could drift too. Running
  every browser file after the rest covers them all, for 53 s.
- **A list of commands in the test.** It would be a third copy. Instead,
  the jobs say which `check.yml` command each one runs, `leftToCi` says
  why each other command isn't run, and the test compares both with the
  workflow.

## Amendment: each part a setting selects runs somewhere (#766, 2026-11-05)

The drift tests held each test file to a job, not each part of one. The
corpus built with std's iterator structs (`ITERATOR_FORMS=corpus`) is a
part of test/iterator-forms.test.ts that only the nightly ran, and #766
reached `main` through it. Now `parts` in `scripts/shard.ts` names each part
a setting selects, with where it runs:
- `push`, by a file that sets it and that a push runs: a workflow with an
  `on: push`, or a script check.yml runs;
- `nightly`, with why.
`harness` names the settings that select no part. A third test in
`test/ci.test.ts` fails on a setting a test file reads that is in neither,
on a part whose file doesn't set it (comments aside), and on a push part
whose file no push runs.

With `--all`, check:fast runs the iterator jobs (both slices and the
corpus with std's structs) where the change or the branch's since `main`
touches `iteratorPaths`, as the push's check does. A push of other code
runs none of them.

## Amendment: the browsers' files run beside the rest, in the pool (2026-11-07)

Since #857, `check:fast`'s jobs run through a pool that starts each once
the cores it keeps busy are free, so the load stays near the cores. The
browsers' files no longer meet the load they starved under, every job at
once, two to five times the cores. Run last, they and the other files'
last ones left the cores idle: on the 12-core M6, a check with no change
kept them 63% busy, its last minute 35%, a third of its time under half.

- **Each test file is a job of the pool's**, the browsers' among them.
  The longest go first, by `target/test-timings.json`, which `bun test
  --timings` keeps too, so no long file runs alone at the end. A group's
  files are said as one line, `every other test file`, `the browsers'
  test files`.
- **Each job holds the cores its last runs measured**, its CPU seconds
  over its own (`Timing.cores`), not the weight it says, which is only
  its first run's: a corpus part said two and kept one, the browsers'
  files said twelve and kept 3.6.
- **The corpus is cut in one part per core** (`pooledParts`), as a part
  keeps one core busy.
- **`after` is gone.** `jobs` returns one list, and the drift tests read
  it.

CI's suite, `scripts/test.ts`, still runs the browsers' files after the
rest, as it runs its files all at once with no pool.

The browsers' files passed in each of the nine `check:fast --all` runs
that ran them beside the rest (2026-11-07), three with an iterator
change's matrix and conformance rows beside them too. With no change, the
check took 2 min 56 s, three runs in a row, the cores 93–94% busy, where
it took 3 min 55 s at 63%. Their timeouts are the
pool's, scaled by its load (`RUST_JS_LOAD_SCALE`).

## Amendment: observe concurrent startup work without blocking (2026-11-15, #1917)

The compiler build and pruning run concurrently. Both exit results must be
known and successful before jobs start. Each duration runs from that child's
own start to observing its exit; the build is awaited asynchronously so
its parent can observe pruning while the compiler is still working.

A synchronous build blocked the event loop that timestamped pruning: a
child that had exited after 17 ms was observed after 323 ms beside a
300 ms blocking step. Cold builds made pruning appear to take 30–43 s.
Those historical numbers cannot establish a disk bottleneck. This fixes
the measurement; it does not claim to speed up the child processes.

The startup boundary takes the two commands and directory explicitly.
A real-process regression uses a build that waits for the prune and then
continues working: startup must remain asynchronous, report the prune's
shorter duration, retain either child's failure, and await both children.
Owner mutations restore blocking, substitute the build duration, skip the
build wait or suppress pruning failure. No test inspects source text.

## Amendment: the jobs start with the build, the prune beside them (2026-11-24, #2498)

The jobs no longer wait for the prune. They need the compiler, so they
start once its build is done; the prune of what the tests keep goes on
beside them, and the kept compiles' prune, which ran in the check's own
process after the build, is a process beside them too. Each is said, with
its time against its usual of the same kind (`, beside the jobs`), once
the jobs have ended, and a prune that failed fails the check as before.
The prune's walk of the stores, sizing each entry, was the critical path of
every check whose build was warm: 25.7 s median on the M1, 6–20 s on the
Mac mini, with the kept compiles' 1–25 s after it.

What it removes beside the jobs is stale: unused since the checkout's last
check and for a day. Its walk takes seconds, so an entry it listed stale
may be used by a job before it's removed; `holdKept` looks at each again
as it goes and keeps one used since (test/kept.ts). Every checkout's prune
already ran beside the others' jobs, and the budget's measure beside its
own; this narrows that race for all of them.

Two cases still wait. Below the disk's floor the jobs would keep nothing
they make, so they start once the prunes have freed what they can. A
build that failed waits for the prune before the check stops, so nothing
of the check's is left running.

Rejected: keeping each store's size in an index, updated by each writer,
so the prune reads only what it removes. With the walk off the critical
path it costs a core for seconds beside 18 busy ones, and an index every
test process writes is a second record of the stores to keep true. It's
#2498's second half, left until a measure says the walk's CPU matters.

Amended the same day: the kept compiles' prune runs again before the jobs, after the build it's of (`startup`'s `compiles`). Beside the jobs, it dropped another build's entries that test/compiles.test.ts keeps in that store and reads again. The ENOENT failed every agent's check, and a false red costs more than its 0.6–25 s. The stores' prune stays beside the jobs. test/check-startup.test.ts holds the order; mutation `check-startup-compiles-pruned-beside`.

Amended the next day (#2498's second half, the lead's call): each kept
store keeps its size in an index after all. Beside the jobs the walk was
14–24 s of a core under load and 9 s quiet (std-reach's 109k entries alone
3.4 s), on every check of every checkout, as the stores are the machine's;
and support.test's floor test walked the machine's stores, 44 s alone on
the Mac mini, before 0f11b7da6 gave it a store of its own. The index is
derived from the walk alone: `.walked.json` beside the store, when
`pruneKept` last walked it and the size it left, and `.kept`, a line for
each entry kept since, its size and path, appended by the store's writer
(`noteKept`, called by each of the five: a native build, a native run, a
verified run, a reach, a native program's sources). Its size between walks
is never less than the store's: a removal by another prune only makes it
more, and a walk takes `.kept` first, so what's kept as it walks is counted
again. A store is walked where it has no index, a day after its last walk
(so what's a week unused goes within a day of it), with the disk below its
floor, or past its cap an hour on (a working set past its cap, which a walk
keeps, isn't walked by every check). A writer that forgets to note is
caught by its test (mutations `native-build-unnoted`, `native-run-unnoted`,
`verified-run-unnoted`, `reach-unnoted`, `native-sources-unnoted`); bytes
missed, as a sources directory's files written after it's made, are found
by the next day's walk.

## Amendment: a push judges the change's mutations CI's check judges, on the tree it pushes (2026-11-25, #2485)

A push judged none of its change's mutations. check:fast's job ran those
the change adds or edits and hasn't committed (`ownSince("HEAD")`), and a
push's change is committed, so the job was empty. CI's check judges
another set, every mutation whose code, list or tests the change touches
(`changedSince`). So agents ran CI's set by hand before pushing, and where
they didn't, #2512 landed with its rule changed twice during its push, its
13 mutations never run again on the tree pushed; #2408's survivors reached
`main` the same way.

The lead's rule (issuecomment-7133196620): a push judges, itself, each
mutation CI's check judges of the change, on the tree it pushes, or it
refuses, naming them. CI's selection, `changedSince`, is the one; no second
is built.

- **check:fast's job** judges `changedSince` of where the change left
  `main`, less those on record for the tree (below), with `--base` of that
  commit: a survivor there too is latent, and passes, as CI's shards rule.
- **The record** (scripts/judged.ts): each mutation run records what it
  judged, caught or latent, by the tree it judged, `treeOf`, the files a
  mutant's workspace is made of (`sourceFilesOf`) written as a git tree, an
  untracked test the change hasn't committed among them. A run whose tree
  changed while it ran records nothing. It's `target/mutants/judged.json`,
  the newest 16 trees.
- **The push's `judged` phase**, each round, after the check: CI's set of
  the change since `origin/main`, less the record of the tree it pushes. Any
  left, where the check was skipped or the tree changed under it, are judged
  there; one still not on record refuses the push. Those that read
  rust-js.wasm are said, and left to CI's wasm job, which judges them.

The cost is CI's set judged on this machine once per tree: a rebase onto a
moved `main` is a new tree, so a push's sanity round judges them again. CI's
mutation shards took 1 to 74 runner-minutes a push over 2026-11-24's last
ten checks, on 4-core machines; here they run in check:fast's pool beside
its tests.

## Amendment (2026-11-25): a rebase carries the round's judging of what it didn't touch

The record is by tree, and a rebase makes a new tree, so each round past
a moved `main` judged the change's whole set again. The lily agent's 2c-1
push (2ab5bb3f9) judged its 351 mutations in round 1's check, then again
in each of three rounds past a moved `main`: 442, 444 and 445 s in the
`judged` phase. None of the incoming commits touched its files, so about
22 min of the push was this.

Now, where a round past a rebase is the same change, its commits patch for
patch those the round before checked (`roundRefused`, as the round's
check is carried, #2484), the mutations judged on that round's tree are
recorded judged on the rebased one (`carryJudged`, scripts/judged.ts).
That holds only for those whose own file, mutation list and named test
files the commits it brought leave alone (`touchedBy`). One the corpus
judges isn't carried past any change to a corpus case, nor one the
compiler's unit tests hold past any Rust change. The round's check:fast
and its `judged` phase then judge the rest.

What the carry risks: a carried verdict can be wrong where an incoming
commit changed what a mutation's tests read beyond those files, a
helper, the compiler elsewhere. CI's check judges the pushed commit's
whole set again (`changedSince` of the check's `since`), so a wrong carry
turns CI red after the push. It never lands a change whose mutations
weren't judged: the lead's acceptance, 2026-11-25.

And push loads no mutation list itself. It asks scripts/judged.ts in a
process of its own (`askJudged`: `files`, `unjudged <base>`, `carry
<checked> <from> <to>`). Each of the 2c-1 push's three rounds ran push
again from the rebased tree because the incoming commits edited mutation
lists that push had loaded (`staleModules`). Now only push's own code, or
a package change, does that.

What holds it: test/judged.test.ts, "a rebase over commits that leave a
judged mutation's file, list and tests alone carries it onto the rebased
tree" and "push loads no mutation list itself". Mutations
`judged-carried-past-its-test`, `-its-list`, `-its-file`, `-a-case`,
`-rust` and `push-lists-loaded-itself`.
