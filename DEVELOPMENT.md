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
| The new mutations, `bun scripts/mutations.ts <name>` | yes, on the Mac | the changed ones | all of them, on 8 machines |
| `bun run check:fast`: lint, the corpus, the lowering, compiler and bindings suites, the snapshots, JSX and diagnostics a change touches, its new mutations, at once | yes, before each commit | | |
| `bun run ci:status`: main's latest run | yes, before each commit: a red one is fixed first | | |
| Reading the generated JS and snapshot diffs | yes | the bless patch, read before it's committed | |
| `bun run typecheck`, `fmt:check`, clippy, `cargo test` | | **lint** job | |
| `bun run test`, the whole suite | | **test** job, or shards | |
| WASM compiler parity | | **wasm** job | |
| rustc's test suite and its lists | | **rustc** jobs, 6 shards | |
| The Vite example's build | | **examples** job | |
| Generated programs (fuzz) | | | yes |
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
`bun install` and `bunx --bun playwright install chromium`; and the apps
above under Developer Tools.

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
  `src/` or the runtime; the mutations, those the change adds or changes;
  CI runs the rest. Each job
  is given the compiler it built, `RUST_JS_COMPILER`: one that builds its
  own links it again, and races the others.
- **`bun run port measure`** fails when the port's compile does: a module
  that fails keeps its JS from before, which reads as no difference.

- **A corpus case's runs are kept by what they ran** (ADR 0567), in
  `target/corpus-cache`: the same JS, native output, runtime modules and
  harness again, its four runs aren't run again; its two compiles always
  are, and its snapshot is always checked. A `check:fast` whose change
  leaves most cases' JS as it was runs the corpus in half the time.
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

## Working on `main`

- **Each change is a commit on `main`, pushed directly**: no branches, no
  pull requests. Its message is titled `feat(rust-js): …`.
- **A push that fails**, GitHub down say, isn't a reason to stop: keep
  committing on `main` locally, and push when it works again.
- **The check comes with the push**: its mutations are those of what
  changed since `main`'s last finished check, or the commit before (its
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

`bun run ci:check` checks another branch, or `main` again; `--shards=4`
splits the suite across four machines.

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
`since`, the commit the mutations are of what changed since. A push's run
waits for the branch's one in progress; one started by hand cancels it.
Its jobs run at once:

| Job | What | Time, about |
|---|---|---|
| `plan` | of a push, the commit its mutations are since, `main`'s last finished check, and their machines | 1 min |
| `lint` | typecheck, fmt, clippy, `cargo test` | 10 min |
| `test` | `bun run test`, or a shard of it, balanced by the last run's timings, `timings-*`; a bless patch on failure | 9 min, 5 in 2 shards |
| `examples` | the Vite example's build | 10 min |
| `wasm` | build `rust-js.wasm`, then the snapshot and playground tests with it | 30 min |
| `rustc` | rustc's suite, a release build, then 6 shards and a report; a lists patch on failure | 20 min |
| `mutations` | `--changed=<since>`: the mutations in a function the change touches, or of a test or corpus case it touches, and those it adds or edits (ADR 0093), on 1–8 machines, about twenty a machine, as the `plan` job counts them | 0–15 min |

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
mutation on 8 machines where a branch's check runs only its own, and 600
generated programs, new ones each night, from the day's number. It's a
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
  whose `timings-*` artifacts `bun scripts/ci.ts timings` fetches.
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
