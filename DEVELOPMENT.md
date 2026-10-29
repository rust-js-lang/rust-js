# Development workflow: short loop here, long checks on CI

Status: in use. [AGENTS.md](AGENTS.md) and [CONTRIBUTING.md](CONTRIBUTING.md)
link here.

## The problem

A change to rust-js is made in one loop: a failing test, the fix, the
mutations that prove the test, the decision record, then **every** check
before the push:

```
test fails ─▶ fix ─▶ test passes ─▶ mutations ─▶ ADR ─▶ FULL CHECK ─▶ commit, push
  seconds     minutes   seconds      minutes             10–20 min, waiting
```

The full check is the whole suite with its blessings, the WASM compiler
rebuilt and checked against the native one, clippy, fmt, the typecheck,
rustc's test suite, and the change's mutations. Run on one machine before
every push, it stops the work while it runs. Most of a change's wall time
is this waiting.

## The idea

Keep two loops.

- **The inner loop** stays here. It's what tells us the change is right:
  the failing test seen to fail, then pass, and the generated JS read. It
  takes seconds to minutes.
- **The outer loop** goes to GitHub Actions. It's what tells us nothing
  else broke: everything else, of each push to `main`. It runs on GitHub's
  machines while we work on the next thing. A red run is fixed at once, on
  `main`, before the next change is built on it.

```
 we (or an agent)               main on GitHub            GitHub Actions
   │                               │                         │
   │ test fails, fix, test passes  │                         │
   │ (inner loop, local)           │                         │
   │ commit on main, push ────────▶│ push starts it ───────▶ │ check  ~10 min
   │                               │                         │  (jobs at once; a newer
   │                               │                         │   push waits its turn)
   │ next change                   │                         │
   │ ...                           │                         │
   │ ◀── bun run ci:status ────────┼─────────────────────────│ before each commit
   │ red: fix on main first, push                            │
   │ bless patch? review, apply, commit, push                │
```

The key fact: a check's result isn't needed to start the next change. So
we don't wait for it; we only come back to it.

## GitHub's machines

The repository is public, so GitHub's machines cost it nothing: every push
to `main` is checked, all of it, the suite on two machines, about ten
minutes. One run at a time of each branch: a newer push waits for the one
running, and of several waiting only the newest runs, so each finished
run's mutations are of what changed since the last finished run, the
skipped pushes' changes too. (Each run cancelling the one before, a push's
or another branch's, left half of `main`'s runs unfinished, at fifty to a
hundred and fifty pushes a day.) A run started by hand cancels the
branch's one running, and no branch's run touches another's.

## What runs where

| Check | Inner loop (local) | Each push to `main` | Nightly, of `main` |
|---|---|---|---|
| The new test, seen to fail, then pass | yes, on the Mac: it's the point | in the suite | in the suite |
| Its module's focused tests, `bun test test/<file>.test.ts -t <name>` | yes, on the Mac | in the suite | in the suite |
| The new mutations, `bun scripts/mutations.ts <name>` | yes, on the Mac | the changed ones | all of them, on 16 machines |
| `bun run check:fast`: lint, the corpus, the lowering, compiler and bindings suites, the snapshots, JSX and diagnostics a change touches, its new mutations, at once | yes, before each commit | | |
| `bun run ci:status`: main's latest run | yes, before each commit: a red one is fixed first | | |
| Reading the generated JS and snapshot diffs | yes | the bless patch, read before it's committed | |
| `bun run typecheck`, `fmt:check`, clippy, `cargo test` | | **lint** job | |
| `bun run test`, the whole suite | | **test** job, or shards | |
| WASM compiler parity | | **wasm** job | |
| rustc's test suite and its lists | | **rustc** jobs, 6 shards | |
| The Vite example's build | | **examples** job | |
| Generated programs (fuzz) | | | yes |
| Each corpus case after each of tidy's stages, `test/tidy-stages.test.ts` | its six cases, in `check:fast` when tidy changes | its six cases, in the suite | every case, on 6 machines |
| Release qualification | | | yes |
| react.dev port: its JS against what it committed, `bun run port measure`; its build's 823 pages, `bun run port compare` | yes: the port is private | | |
| How the generated JS reads, `test/readability.test.ts` against `docs/readability.txt` (ADR 0565) | with `bun run bless` | in the suite | in the suite |

Every local run is five minutes at most
([AGENTS.md](AGENTS.md#five-minutes-per-local-command)); what may take
longer is CI's.

## The Mac, with the scan off

macOS checks each newly built binary before its first run, one at a time,
whatever the number of cores. That check, not the Mac, is what made the
checks that build native programs slow there. It can be turned off for the programs one app starts: add the app
under **System Settings → Privacy & Security → Developer Tools**, then quit
and reopen it.

For the Claude desktop app, the app that starts commands isn't `Claude.app`
itself, but Claude Code inside it, which a helper makes responsible for what
it runs:

```
Claude.app
  └─ Contents/Helpers/disclaimer      hands responsibility to its child
       └─ claude.app                  ~/Library/Application Support/Claude/
            └─ zsh, bun, the tests       claude-code/<version>/<hash>/claude.app
```

So add both: `/Applications/Claude.app`, and that `claude.app` (press ⌘⇧G in
the file picker to paste its path). Add a terminal you run tests from too,
Ghostty or iTerm. The folder has Claude Code's version in its name, so
after an update, measure again; if first runs are slow again, add the new
one.

Eighty small programs, built and run at once, on an M3 Max (14 cores):

| | Scanned | Scan off |
|---|---|---|
| Their first runs | 15 000 ms | 26 ms |
| Building them | 1 400 ms | 1 300 ms |
| Forty of them, built and run | 7 550 ms | 720 ms |
| Starting one process | 2.4 ms | 1.9 ms |

So with the scan off, every local check runs on the Mac: the failing test,
a module's tests, mutations, the corpus, a folder of rustc's tests. No
other machine, no sync, and the files stay ours to edit.

The trade: whatever these apps start isn't checked for malware, including
what an agent runs. Turn it off in the same place.

To set the Mac up once: rustup, with the toolchain and components
[rust-toolchain.toml](rust-toolchain.toml) pins, and the `wasm32-wasip1`
and `wasm32-unknown-unknown` targets; the Bun `package.json`'s
`packageManager` names, the one place its version is; Node 24; then
`bun install` and `bunx --bun playwright install chromium
chromium-headless-shell`, again whenever `bun install` updates Playwright,
or the pilot's and the browser tests fail to launch (`Executable doesn't
exist`); and the apps above under Developer Tools.

### A compile cache shared by the worktrees: sccache

Each agent's worktree has a `target/` of its own, so each first builds
every crate rust-js depends on again, oxc and the rest, minutes of CPU
the other worktrees already spent. [sccache](https://github.com/mozilla/sccache)
keeps what rustc wrote for a crate and gives it to the next build that
asks for the same crate, compiled the same way. Set it up once per machine:

```
brew install sccache          # or: cargo install sccache --locked
```

and in `~/.cargo/config.toml`, the user's, not the repository's:

```toml
[build]
rustc-wrapper = "/opt/homebrew/bin/sccache"   # `which sccache`
```

`sccache --show-stats` says what it hit; its cache is 10 GB by default
(`SCCACHE_CACHE_SIZE`), in `~/Library/Caches/Mozilla.sccache` on macOS.

Why it can't change what rust-js writes:

- Its key is the crate's sources, rustc's binary and version, every flag,
  the target and the environment rustc reads: any difference compiles anew.
- rust-js's own compile of a crate to JS doesn't go through it. Cargo runs
  rust-js as `RUSTC_WRAPPER` from the environment (`tooling/cargo.js`),
  which wins over the config's wrapper.
- A crate built incrementally, rust-js itself in a debug build, isn't
  cached: sccache counts it "non-cacheable" and rustc compiles it.
- CI doesn't use it, so CI is the check that the cache changed nothing.

What a cache bug would look like, and what to do: a build or link that
fails in one worktree and not another from the same commit, a native
crate's error that names a file or line that isn't there, or a local
failure CI's run of the same commit doesn't have. Build again without the
cache, `RUSTC_WRAPPER= cargo build`, and if that passes, the cache was
the cause: clear it, `sccache --stop-server && rm -rf
~/Library/Caches/Mozilla.sccache`, and file an issue titled "sccache: …"
with both outputs, so every agent knows. Turn it off for good by removing
the `rustc-wrapper` line.

## What a step of the loop costs

On the M3 Max, with the scan off:

| Step | Takes |
|---|---|
| rust-js compiling a small file | 35 ms |
| `cargo build` after an edit | 4 s |
| A focused test, `bun test test/<file>.test.ts -t <name>` | 0.3 s |
| One mutation | 2-5 s |
| Eight mutations, four at a time | 11 s |
| The react.dev port, built | 30 s |
| `bun run check:fast`, of a change to the lowering | 40 s |
| `bun run check:fast --all`, JSX, snapshots and diagnostics too | 40 s |
| `bun run port measure`, react.dev compiled against its committed JS | 7 s |
| `bun run port compare`, its build and 823 pages | 7–15 s |

Each of them is timed as it runs, and kept, in `target/timings.jsonl`
(`scripts/timing.ts`): `check:fast`, its build and each of its jobs,
`port measure` and `compare`, a mutation, each, and, read by `bun run
ci:status`, each green check of `main`. A step says its time against its
usual, the middle of its last ten runs, and warns where it's a quarter and
five seconds slower:

```
ok   check:fast bindings 26 s (usual 25 s)
ok   check:fast corpus 1/3 52 s (usual 30 s)  ⚠ slower than usual: why?
```

`bun run timings` shows every step, its last time, its usual and its
slowest; `bun run timings <step>` its runs, by commit. A warning is the
next thing to look at: what made it slower, before the loop is built on it.

What keeps them there:

- **`check:fast` runs its checks at once**, each a process of its own, and
  the corpus in parts, one for each two cores (`RUST_JS_CASES=i/n`,
  test/support.ts): one after another they took four minutes. The corpus,
  lowering, compiler and bindings tests always run; the JSX tests where the
  change touches JSX, the snapshot and diagnostic tests where it touches
  `src/` or the runtime, and with them the guide's examples, std's package
  and the readability ratchet, which a lowering change turned `main` red
  with when only CI ran them; the mutations, those the change adds or
  changes; CI runs the rest. Each job
  is given the compiler it built, `RUST_JS_COMPILER`: one that builds its
  own links it again, and races the others.
- **The node bindings are built before the jobs**, by support.ts's
  `buildNode`, whose stamp skips it where nothing changed: built by the
  first test that asked, under the corpus's load, it outran bun's 5 s
  test timeout, was killed, and failed every test waiting on it, nearly
  every run. Tests that run `cargo` themselves declare a timeout that fits.
- **The snapshot tests declare 60 s**: `inbox`'s compile, a crate of
  serde's, outran bun's 5 s under load, nearly every loaded run, and in
  CI's `wasm` job twice running (2026-10-27).
- **`bun run port measure`** fails when the port's compile does: a module
  that fails keeps its JS from before, which reads as no difference.

- **A corpus case's runs are kept by what they ran** (ADR 0567), in
  `target/corpus-cache`: the same JS, native output, runtime modules and
  harness again, its four runs aren't run again; its two compiles always
  are, and its snapshot is always checked. A `check:fast` whose change
  leaves most cases' JS as it was runs the corpus in half the time.
  What no test used for a week is pruned by `bun run test`, and by
  `check:fast` once a day (`target/kept-pruned`), as only CI runs the
  whole suite: a Mac's `target/native-cache` grew 23 GB in two days.
- **The react, webapi and js crates' metadata** is built only when their
  sources, build scripts or the toolchain change: a stamp beside it,
  `target/libreact.stamp`, says what it was made from. Built every run, it
  was three seconds of a focused test's four.
- **rust-js is built with one `RUSTC_BOOTSTRAP`**, `.cargo/config.toml`'s,
  by the tests as by a shell. `.env.test` gives the tests' programs `1`,
  and a build with it rebuilt every dependency, 22 s, and the next one
  without it again.
- **`@rust-js/std` is generated**, as `runtime/index.js` is: `bun run std`
  after a change to what std's bodies lower to, then a rebuild, which
  builds its manifest in; `test/std-package.test.ts` holds it (ADR 0561).
- **Mutations keep their crate** in `target/mutants/crate`, writing only
  the files that differ, and the compiler built without a mutation while
  the sources are what it was built from. They run up to four at a time,
  one worker for each two cores (on this Mac 1 took 70 s for sixteen, 2
  took 43 s and 4 took 31 s; CI's four cores run two), each with a crate
  and a target of its own (`--jobs=N` says how many). Each one's build and
  tests are timed: CI's summary shows them, and each shard uploads
  `mutation-timings-<shard>`. A runtime helper's mutation, whose tests the
  checkout's `runtime/index.js` serves, runs alone after the others.

## Keeping the loop fast

The loop is part of the product: a slow step is paid on every change, by
every agent at once. Noticing one and making it faster is everyone's job,
as a wrong answer is.

- **Use the loop's own tools, so its timings see you.** `bun run
  check:fast` is the change's checks at once, timed against their usual;
  hand-rolled runs (a corpus shard in the background, a build then a test)
  are timed by nobody, and their slowness goes unseen.
- **Read a `⚠ slower than usual` line before going on**: find what made
  it slower, fix it or write it in the list below.
- **When a step is slower than it needs to be**, fix it if it's yours to
  fix: a cache, a stamp, a parallel run, a smaller test. Otherwise add it
  to the list below, with what it costs and what would fix it, so the next
  agent starts from it. A fix moves its line to "What keeps them there",
  above, or out.
- **Build what the tests use.** The tests, `check:fast` and an app built
  from this checkout run the debug compiler, `target/debug/rust-js`
  (`bun run build`); `cargo build --release` builds one nothing here runs.
- **Run few agents at once.** Each worktree builds its own `target/`
  (about 4 GB) and runs the corpus on every core; four or five at a time
  made every one of them two or three times slower, and timed-out tests
  look like failures. Three at a time kept each near its usual.
- **Keep the Mac awake** while work runs unattended (`caffeinate -dims` in
  a terminal): a sleeping Mac pauses every process, and `timeout` with
  them, so a five-minute command took hours and its tests failed on time.
- **Never search the whole disk** (`find /`): it ran sixteen minutes and
  slowed every agent. A dependency's source is where Cargo keeps it: a
  crates.io one under `~/.cargo/registry/src/*/<name>-<version>`, a git
  one, as oxc is, under `~/.cargo/git/checkouts/<repo>-*/<rev>/`.

### Landing a branch on a moving `main`

Several agents land at once, so `main` often moves while a change is being
checked:

- **Rebase, never merge `main` in**: `git fetch`, `git rebase
  origin/main`. If `main` was force-pushed (`forced update`, an author
  rewrite), `git rebase --onto origin/main <old tip>`, so the old commits
  don't come back.
- **See the rebase finish before checking**: `git status` says `rebasing`
  while it waits on a conflict, and checks run then test a half-applied
  branch, which passes or fails for no reason of its own.
- **See the push land before cleaning up**: `git push` is refused when
  `main` moved meanwhile, and a quiet `-q` push says so in one line that's
  easy to miss. Compare `git rev-parse HEAD` with `origin/main` after a
  `git fetch` before removing the worktree that made the change; its
  branch stays, but the next step starts from the wrong place.
- **Regenerate what's generated on a conflict**, never merge it by hand:
  `docs/readability.txt` with `BLESS=1 bun test test/readability.test.ts`
  (its shares are compared over the files it lists, so a case added is no
  reason to bless: it's counted once anything else is blessed),
  corpus snapshots with `bun run bless`; the decision index merges by
  union, and the architecture test names a line left twice.
- **After a rebase, check again only what the incoming commits touch**:
  their `src/` changes need the change's checks again; docs and other
  agents' tests don't. A full re-run for every commit that landed meanwhile
  is a loop that never ends.
- **ADR numbers**: `bun run adr:next`, and `bun run adr:renumber` when
  `main` took yours.

### Slow spots, still to fix

| Step | What it costs | What would fix it |
|---|---|---|
| `bun scripts/mutations.ts --changed=origin/main` of a large change | More than five minutes even in three shards, so it's left to CI | Shards picked to fit, by each mutation's usual time |
| Two or more agents' worktrees | A full `cargo build` and corpus each | A shared build cache across worktrees |
| Clippy's crates in the compiler, for `rust-js clippy` (ADR 0888) | A cold `cargo build` builds them first: 12 s (29 s of CPU) of its 59 s on the M3 Max, and 330 MB more of `target/`; the debug binary is 104 MB, not 59 MB. A build after an edit is as before, 3.5 s | Built once for every worktree, the shared build cache above |
| `check:fast`'s bindings job while other worktrees build | Failed once at 35 s and passed alone, 75 of 75 | Find what it races on |
| macOS's `xprotectd` scanning freshly built binaries and fixtures | Load near 20; every corpus case and suite timed out at about 270 s against 30–50 s, passing once it settled | The scan off for the apps that build ([The Mac, with the scan off](#the-mac-with-the-scan-off)); check `ps` for `xprotectd` before believing timeouts |
| A sweep of generated programs against native Rust, by hand | 4–5 min for 1,100–2,100 programs, again after each fix, each a `rustc`, a rust-js compile and a run | A checked-in runner: native outputs kept by source, only rust-js's compile and run again, verdicts said against the last sweep (fixed, regressed) |
| `test/architecture.test.ts`'s "a binding's optional argument is an Option of its one function" under load | Times out at bun's 5 s in some loaded `check:fast` runs (5.2 s), a re-run each time | A timeout that fits its compile |
| A compile of std's bodies, `bun run std` and `test/std-package.test.ts` | 8.5 s alone on the M3 Max (6.2 s before `Filter` and the adapters' `try_rfold`, ADRs 0930, 0932; 5.4 s before `StepBy`; 1.3 s before the iterator plan, ADR 0908), past 60 s beside `check:fast`'s other jobs, so its timeout is 120 s; each mutation of a std body's lowering compiles it again | The debug compiler is the cost: a release build of the compiler for the std build, or the std package compiled once per compiler and reused by the tests that only read it |
| A mutation that makes the compiler never finish, `drop-of-its-own-type-inlined` | Caught only by its corpus case's timeout: about a minute where its change's `check:fast` runs it, and in the nightly | A compile timeout of its own for a corpus case that says it, a few seconds |
| Two tests that compile a port inside bun's 5 s, lowering-labels' "React's addObjectDiffToProperties has no block MIR made" and js-builtins' "unwrap of a throwing call each branch makes" | Time out in `check:fast` when other worktrees load the Mac (load 35–48), failing a run that passes with each alone (8 s for the second's file) | A timeout that fits their compile, or the port compiled once per run and shared |
| `check:fast`'s guide, std package and readability job | The guide's examples take every core for ~20 s, and the other jobs slow (architecture's 6 s to 13 s) | The guide's examples kept by what they compiled, as the corpus's runs are |

## Working on `main`

- **Each change is a commit on `main`, pushed directly**: no branches, no
  pull requests. Its message is titled `feat(rust-js): …`.
- **A push that fails**, GitHub down say, isn't a reason to stop: keep
  committing on `main` locally, and push when it works again.
- **The check comes with the push**: its mutations are those of what
  changed since `main`'s last check whose mutations passed, or the commit before (its
  `plan` job). A red run is fixed at once, with another commit.
- **One run at a time of each branch.** A push's check waits for the one
  running; of several waiting, the newest runs. A check started by hand,
  `bun run ci:check` or a `rustc tests` run, cancels the branch's one
  running: the latest is what's wanted of it. Another branch's run is
  neither waited for nor cancelled.

## When CI writes something: bless patches

Some checks rewrite files: snapshots (`bun run bless`) and rustc's lists of
known failures (`bun run test:rustc:bless`). CI never pushes.
A job whose check fails on such a file runs the bless too, and uploads
what changed as a patch, a `bless-*` artifact. We read it as we'd read a
local bless, and apply it:

```bash
bun run ci:bless            # downloads main's latest patches, git apply
git diff --cached           # review every snapshot change, as always
git commit -m "chore(rust-js): bless"; git push
```

## Day to day

```bash
#   on main: failing test, fix, focused tests, new mutations, ADR
bun run check:fast                       # the change's checks, at once
git commit -m "feat(rust-js): …"
git push                                 # starts main's check, on GitHub
#   next change, while it runs
bun run ci:status                        # before each commit: green, red or running, and what failed
gh run view <id> --log-failed            # why
bun run ci:bless                         # apply its bless patches, review, commit, push
```

`bun run ci:check` checks another branch, or `main` again, its suite on
the workflow's 2 machines; `--shards=4` splits it across four.

## For agents

- **Push, and move on; read main's run before the next commit.** The push
  starts the check. Don't wait for the run, poll it, or schedule a check of
  it: read it with `bun run ci:status` before each commit, and fix a red
  run first, so nothing is built on what's broken.
- **Five minutes per local command, at most** ([AGENTS.md](AGENTS.md#five-minutes-per-local-command)):
  each is run with a timeout; what may take longer is CI's.
- **The inner loop is never delegated.** The failing test is seen to fail
  here, before the fix, and its mutations caught here, before the push.
- **Read every bless patch before committing it.**
- **A red run is the change's.** Fix it at once, on `main`, and say what
  broke, with the failing job's log.

## Workflows

### `check.yml`, of each push to `main`, and `bun run ci:check`

Each push to `main`, its suite on 2 machines; or `workflow_dispatch` of a
branch, by `bun run ci:check`, with `test_shards`, 1, 2 or 4, and
`since`, the commit the mutations are of what changed since. A run waits
for the branch's one in progress, a push's and one started by hand alike,
and never cancels it: of several waiting, the newest runs, its mutations
of everything since `main`'s last check whose mutations passed. A push to `main` checks
it already; start one by hand only of another branch, or with other
settings. `rustc-tests.yml`'s runs are a group of their own, so one never
cancels a check.
Its jobs run at once:

| Job | What | Time, about |
|---|---|---|
| `plan` | of a push, the commit its mutations are since, `main`'s last check whose mutations passed, and their machines | 1 min |
| `lint` | typecheck, fmt, clippy, `cargo test` | 10 min |
| `test` | `bun run test`, or a shard of it, balanced by the last run's timings, `timings-*`, the corpus in parts beside it; a bless patch on failure | to be measured: 13–24 min in 2 shards with the corpus in one file |
| `examples` | the Vite example's build | 10 min |
| `wasm` | build `rust-js.wasm`, then the snapshot and playground tests with it | 30 min |
| `rustc` | rustc's suite, a release build, then 6 shards and a report; a lists patch on failure | 20 min |
| `mutations` | `--changed=<since>`: the mutations in a function the change touches, or of a test or corpus case it touches, and those it adds or edits (ADR 0093), on 1–16 machines, about twenty a machine, as the `plan` job counts them | 0–15 min |

### `rustc-tests.yml`, by hand, and called by `check.yml`

As before, rustc's tests, named or all, blessed or checked; or generated
programs. Its `mutations` take `mutation_args`, names or `--changed=..`,
and `mutation_shards`, 1, 2, 4 or 8. Before a release, all of them:

```bash
gh workflow run "rustc tests" -f mutations=true -f mutation_shards=8
gh workflow run "rustc tests" -f fuzz_seeds=600 -f fuzz_start=1000
gh workflow run "rustc tests" -f bless=true    # rustc's lists, as a bless patch
```

### `nightly.yml`, each night at 02:00 UTC+7, of `main`

Every test there is: `check.yml`'s jobs, its suite in 2 shards and every
mutation on 16 machines where a branch's check runs only its own, 600
generated programs, new ones each night, from the day's number, and every
corpus case after each of tidy's stages on 6 machines (ADR 0659). It's a
group of its own, so it neither cancels nor waits for a push's check.
`bun run ci:status` shows its latest run first; GitHub mails a failure to
whoever last changed the schedule. Of a public repository, its machines
cost nothing. Started by hand too: `gh workflow run nightly.yml`.

### Kept as they are

`qualify.yml`, `publish-npm-crates.yml` and `deploy-guide.yml`, which
builds the guide's site and deploys it to GitHub Pages (`bun run deploy`).

## What it's made of

- `.github/workflows/check.yml`, the jobs above, with caches: Cargo's
  registry and `target/debug` by `Cargo.lock` and the toolchain; the WASM
  build's by its lockfile and patches.
- `scripts/test.ts --shard=i/n`, which `bun test --shard` splits by how
  long each file took, in this checkout and in the last run's shards,
  whose `timings-*` artifacts `bun scripts/ci.ts timings` fetches; and
  the corpus beside them in parts, one for each two cores, as
  `check:fast` runs it, each shard its `1/n` of them (`caseShardParts`,
  `scripts/shard.ts`). One file of 1,300 cases one after another, it was
  13 to 24 minutes of its shard's time on CI, the rest done in 4, and
  stopped at the suite's 25-minute deadline (2026-10-29).
- `scripts/mutations.ts --changed=<base>` and `--shard=i/n`, by
  `scripts/shard.ts`.
- `scripts/ci.ts`: `bun run ci:check`, `ci:status`, `ci:bless`, and a
  push's `plan`.
- `scripts/port.ts`: `bun run port measure` and `compare`, of the
  react.dev port beside this checkout.
- `scripts/timing.ts`: each step's time kept, said against its usual, and
  `bun run timings`.
- `test/ci.test.ts`: the shards, the changed mutations, each branch's
  latest run.

## Costs and limits

- **Results come later.** A red run brings us back to a change we'd left.
  Small changes on their own branches keep that cheap.
- **Minutes are counted**, so a check is started when a change is ready,
  not on each push.
- **GitHub's machines are slower than the Mac**, 4 cores against 14. It
  doesn't matter, as nothing waits on them.
- **x86 against Apple Silicon, Linux against macOS.** CI runs on x86 Linux,
  the Mac on arm64 macOS. A test either one ignores is out of scope, and
  rustc's known failures are Linux's, so CI blesses their lists.
- **The react.dev port stays local.** It has no remote, so its build and its
  823 pages compared stay in its own loop.
