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
