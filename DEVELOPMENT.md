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
  else broke: everything else, once a branch is ready, started by hand,
  `bun run ci:check`. It runs on GitHub's machines while we work on the
  next thing. `main` only takes a branch whose check is green.

```
 we (or an agent)            branch on GitHub          GitHub Actions           main
   │                               │                         │                    │
   │ test fails, fix, test passes  │                         │                    │
   │ (inner loop, local)           │                         │                    │
   │ commit, push ────────────────▶│                         │                    │
   │ bun run ci:check ─────────────┼───────────────────────▶ │ check  ~30–40 min  │
   │                               │                         │  (jobs at once)    │
   │ next change, next branch      │                         │                    │
   │ ...                           │                         │                    │
   │ ◀── bun run ci:status ────────┼─────────────────────────│                    │
   │ red: fix on that branch, push, check again              │                    │
   │ bless patch? review, apply    │                         │                    │
   │ green: merge ─────────────────┼─────────────────────────┼───────────────────▶│
```

The key fact: a check's result isn't needed until the merge. So we don't
wait for it; we only come back to it.

## GitHub's minutes

GitHub gives the repository a number of free minutes of its machines, and
every job of a run counts its own, setup included. So nothing runs on its
own: no run on each push, none on a schedule. A branch is checked when
it's ready to merge, once, and again only after a fix. A check costs about
the sum of its jobs, some 2 to 3 hours of machine time with rustc's
shards; splitting the suite across more machines ends sooner but costs
more, as each sets itself up again.

## What runs where

| Check | Inner loop (local) | `bun run ci:check` | By hand, before a release |
|---|---|---|---|
| The new test, seen to fail, then pass | yes, on the Mac: it's the point | in the suite | in the suite |
| Its module's focused tests, `bun test test/<file>.test.ts -t <name>` | yes, on the Mac | in the suite | in the suite |
| The new mutations, `bun scripts/mutations.ts <name>` | yes, on the Mac | the changed ones | all 919 |
| Reading the generated JS and snapshot diffs | yes | the bless patch, read before merging | |
| `bun run typecheck`, `fmt:check`, clippy, `cargo test` | | **lint** job | |
| `bun run test`, the whole suite | | **test** job, or shards | |
| WASM compiler parity | | **wasm** job | |
| rustc's test suite and its lists | | **rustc** jobs, 6 shards | |
| The Vite example's build | | **examples** job | |
| Generated programs (fuzz) | | | yes |
| Release qualification | | | yes |
| react.dev port: build and the 823 pages compared | yes: the port has no remote | | |

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

## Branches, pull requests, merging

- **One branch per change**, from `main`, or from the branch it builds on.
  Name it for the change: `untagged-otherwise`, `editor-check`.
- **Each branch is a pull request**, ready for review, never a draft,
  titled as a commit is: `feat(rust-js): …`. A pull request gives the
  checks one place, `gh pr checks`, and lets the Claude desktop app watch
  them and say when they're done.
- **`main` takes only a branch whose check is green**, by a rebase: `gh pr
  merge --rebase`. With checks started by hand, GitHub can't require them
  of every push, so this is ours to keep.
- **Changes stack.** A change that needs one not yet merged starts from its
  branch. When the earlier one goes red, fix it there and rebase the later
  ones on it. When it merges, rebase them on `main`.
- **One run at a time.** A new check, of any branch, or a `rustc tests`
  run started by hand, cancels the one running: the latest is what's
  checked, and two never spend minutes at once.

## When CI writes something: bless patches

Some checks rewrite files: snapshots (`bun run bless`) and rustc's lists of
known failures (`bun run test:rustc:bless`). CI never pushes to a branch.
A job whose check fails on such a file runs the bless too, and uploads
what changed as a patch, a `bless-*` artifact. We read it as we'd read a
local bless, and apply it:

```bash
bun run ci:bless            # downloads the branch's latest patches, git apply
git diff --cached           # review every snapshot change, as always
git commit -m "chore(rust-js): bless"; git push
```

## Day to day

```bash
git switch -c editor-check main          # or the branch it builds on
#   inner loop: failing test, fix, focused tests, new mutations, ADR
git push -u origin HEAD
gh pr create --fill                      # ready for review, not a draft
bun run ci:check                         # the branch's check, on GitHub
#   next change, on its own branch, while it runs
bun run ci:status                        # each branch: green, red or running, and what failed
gh run view <id> --log-failed            # why
bun run ci:bless                         # apply its bless patches, review
gh pr merge --rebase --delete-branch     # when green
```

`bun run ci:check --shards=4` splits the suite across four machines: done
sooner, at more minutes.

## For agents

- **Check once a branch is ready, and move on.** `bun run ci:check` after
  the push that completes the change, not after every push. Don't wait for
  the run, poll it, or schedule a check of it: read it with `bun run
  ci:status` when the desktop app says it's done, or when asked.
- **Five minutes per local command, at most** ([AGENTS.md](AGENTS.md#five-minutes-per-local-command)):
  each is run with a timeout; what may take longer is CI's.
- **The inner loop is never delegated.** The failing test is seen to fail
  here, before the fix, and its mutations caught here, before the push.
- **Merge only green, and only after reading every bless patch.**
- **A red run is the change's.** Fix it on its branch, and say what broke,
  with the failing job's log.

## Workflows

### `check.yml`, started by `bun run ci:check`

`workflow_dispatch` of a branch, with `test_shards`, 1, 2 or 4. A new run,
of any branch, cancels the one in progress. Its jobs run at once:

| Job | What | Time, about |
|---|---|---|
| `lint` | typecheck, fmt, clippy, `cargo test` | 10 min |
| `test` | `bun run test`, or a shard of it; a bless patch on failure | 30 min, or less in shards |
| `examples` | the Vite example's build | 10 min |
| `wasm` | build `rust-js.wasm`, then the snapshot and playground tests with it | 30 min |
| `rustc` | rustc's suite, a release build, then 6 shards and a report; a lists patch on failure | 20 min |
| `mutations` | `--changed=origin/main`: the mutations whose file the change touches, and those it adds or edits | 0–30 min |

### `rustc-tests.yml`, by hand, and called by `check.yml`

As before, rustc's tests, named or all, blessed or checked; or generated
programs. Its `mutations` take `mutation_args`, names or `--changed=..`,
and `mutation_shards`, 1, 2, 4 or 8. Before a release, all of them:

```bash
gh workflow run "rustc tests" -f mutations=true -f mutation_shards=8
gh workflow run "rustc tests" -f fuzz_seeds=600 -f fuzz_start=1000
```

### Kept as they are

`qualify.yml`, `publish-npm-crates.yml` and `deploy-playground.yml`.

## What it's made of

- `.github/workflows/check.yml`, the jobs above, with caches: Cargo's
  registry and `target/debug` by `Cargo.lock` and the toolchain; the WASM
  build's by its lockfile and patches.
- `scripts/test.ts --shard=i/n`, which `bun test --shard` splits by how
  long each file took.
- `scripts/mutations.ts --changed=<base>` and `--shard=i/n`, by
  `scripts/shard.ts`.
- `scripts/ci.ts`: `bun run ci:check`, `ci:status` and `ci:bless`.
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
