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
| `bun run check:fast`: lint, the corpus, the lowering, compiler and bindings suites, the snapshots, JSX and diagnostics a change touches, each file a job, its new mutations, at once; of a change the test map can say, only the files and corpus cases that ran a function it changes (ADR 1101); `--all`, what a push's check runs that can run here: typecheck, fmt, clippy, `cargo test`, the Vite example's build, and every test file of the suite, the browsers' after the rest | yes, before each commit; `--all` before a push, with the change's mutations: nothing else | | |
| `bun run ci:status`: main's latest run | yes, before each commit: a red one is fixed first | | |
| `bun run nightly`: every job the nightly runs, here, each in parts on all the cores (`scripts/nightly.ts`); `bun run nightly stages forms` runs some | yes, before a change the nightly would find, or to prove a night green | | |
| Reading the generated JS and snapshot diffs | yes | the bless patch, read before it's committed | |
| `bun run typecheck`, `fmt:check`, clippy, `cargo test` | in `check:fast --all` | **lint** job | |
| `bun run test`, the whole suite but the nightly's files | in `check:fast --all`, the browsers' files after the rest | **test** job, or shards | with them, `--nightly` |
| The files only the nightly runs, `nightly` in `scripts/shard.ts` (ADR 1030): the release's packaging, the coverage ratchets, React's releases, the editor's check, the harness's own, build history | `bun test test/<file>.test.ts` when a change touches what one checks | | in the suite |
| WASM compiler parity | | **wasm** job | |
| rustc's test suite and its lists | | **rustc** jobs, 6 shards | |
| The Vite example's build | in `check:fast --all` | **examples** job | |
| Generated programs (fuzz) | | | yes, and the suite's 12 seeds |
| Each corpus case after each of tidy's stages and without each of preparation's rewrites that changed it, `test/tidy-stages.test.ts` (ADR 1087) | its nine cases, in `check:fast` when tidy, preparation or the oracles change, `--all` too, readability-passes.yml's paths (#998): 31–38 s alone on the M1, 51 s beside the corpus, its runs kept and each case's variants compiled at once (#808) | its nine cases, the read cases and six generated programs, **Readability passes** workflow, of a push that changes tidy or preparation | its nine cases in the suite; every case, on 6 machines |
| The day's 600 generated programs through the same, `TIDY_STAGES=programs` | | | yes (`programs`, 6 machines of 100) |
| What JS made, read as often as Rust reads it in every variant, `test/getter-reads.test.ts` (ADR 1087) | in `check:fast` when tidy or preparation changes: 16 s on the Mac | with the stages' sample | in the suite |
| The iterator conformance matrix, `test/iterator-matrix.test.ts` (ADR 1090): every source, chain, consumer and panic site in both builds against native Rust, and closures that capture what drops, and a type stepped from its back beside its forward consumers | its slice, 64 fixed batches, `bun scripts/iteratorSlice.ts run`, in `check:fast` when a change touches the iterators' code (`iteratorPaths`) and in `--all`, beside the rows of both tables the change names, `bun scripts/iteratorSlice.ts run rows` (`scripts/iteratorRows.ts`, #818), which says hunk by hunk what it picked: 119 s alone in 4 parts at a load of 20; `ITERATOR_MATRIX=all RUST_JS_CASES=i/n` for all of them | the slice, **check**'s `iterators` job, where `plan` says the change is the iterators' code | every case, on 6 machines (`matrix`) |
| The iterator conformance table, `ITERATOR_TABLE=conformance` of the same test (ADR 1092): each adapter alone through each step of the contract, from either end, with closures that hold a destructor, in both builds against native Rust | its slice, 43 fixed batches, with the matrix's by `bun scripts/iteratorSlice.ts run`, as the matrix's; `ITERATOR_TABLE=conformance ITERATOR_MATRIX=all` for all 360 | its slice, check's `iterator-conformance`, beside the matrix's | every case, on 2 machines (`conformance`) |
| The iterator panic points, `ITERATOR_TABLE=points` of the same test (#817): each of the conformance table's cases run once each point a panic can be, each closure call, drop, clone and step of the crate's iterator, in both builds against native Rust | `ITERATOR_TABLE=points ITERATOR_MATRIX=all RUST_JS_CASES=i/n` for all 105 batches, 48 s in 10 parts | none yet: #818's gate decides | every case, on one machine (`points`) |
| The unwind table, `ITERATOR_TABLE=unwind` of the same test (#854, `test/unwind-points.ts`): each operation beyond iterators, a value's fields, closures, collections' operations that call the crate's code, `Option` and `Result` combinators, comparators and `?`, of each shape of value, run once each point a panic can be, in both builds against native Rust | `ITERATOR_TABLE=unwind ITERATOR_MATRIX=all RUST_JS_CASES=i/n` for all 58 batches, 27 s in 10 parts | none yet: #818's gate decides | every case, on one machine (`unwind`) |
| Release qualification | | | yes |
| react.dev port: its JS against what it committed, `bun run port measure`; its build's 823 pages, `bun run port compare` | yes: the port is private | | |
| React's port, react-in-rust at `ports/react-in-rust.pin`: `cargo check` of it, `bun scripts/react-in-rust.ts check` | when a change touches a binding | **React port** workflow, of a push that changes `builtins/`, `webapi/`, `node/` or `react/` | |
| React's port compiled again, its JS against what the pin committed, `bun scripts/react-in-rust.ts regen` | when a change would move React's JS | | yes (`port`) |
| The crates of crates.io's corpus against `docs/crate-corpus.md`, `bun scripts/crate-corpus.ts --check [name ..]` | when a change touches what a crate uses | | yes (`crates`) |
| The small Crater, each corpus crate's own `#[test]`s natively and as JS, `bun scripts/crater.ts --check [name ..]` (ADR 1109) | when a change touches what a crate's tests use | | yes (`crater`) |
| How the generated JS reads, `test/readability.test.ts` against `docs/readability.txt` (ADR 0565) | with `bun run bless` | in the suite | in the suite |

Every local run has a timeout that fits it, about twice its usual time
([AGENTS.md](AGENTS.md#making-and-checking-changes)), and uses all the
cores: what's quicker here than CI's round trip runs here.

## The Mac, with the scan off

macOS checks each newly built binary before its first run, one at a time,
whatever the number of cores. That check, not the Mac, is what made the
checks that build native programs slow there. It can be turned off for the programs one app starts: add the app
under **System Settings → Privacy & Security → Developer Tools**, then quit
and reopen it.

**Which app to add, on any host:** the binary just below `launchd` in the
chain of parents of the shell that runs the commands. Walk up with
`p=$$; while [ "$p" -gt 1 ]; do ps -o ppid=,comm= -p $p; p=$(ps -o ppid= -p $p); done`.
Run from a terminal, that's the terminal (`Ghostty.app`), or a multiplexer
it starts (`herdr`, `~/.local/bin/herdr`); add each one a session may run
under. **To check it's off:** build twenty small programs with `rustc` and
time their first runs against their second. Scanned, the first runs took
6 s together (300 ms each); off, 0.05 s, the same as the second runs
(2026-11-05). Check again after each update of the app.

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

What it can't share: a build whose target is `CARGO_TARGET_DIR` in the
environment misses every crate, as sccache's key holds rustc's `CARGO_*`
variables, so a script gives Cargo its target as `--target-dir` (#932);
and in each new target, the 56 of the compiler's 146 dependencies that
are, or are built with, a proc macro, whose dylibs differ by target. A
fresh worktree's `cargo build` is 43 s on the 12-core M6, 27 s of it the
compiler's own crate.

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

### The tests' compiles to JS, kept by their inputs

Each compile to JS a test makes through `test/child.ts`, the corpus's,
`run([compiler, ..])`'s and the rest, is kept in `target/compile-cache/`
and given again where all it's made of is the same (`test/compiles.ts`,
ADR 1099, #874): the compiler's build (ADR 1003), its arguments, where
it ran, its environment, and every file its record of the build
(`--emit=dep-info`) says it read, by content, the crates it loaded among
them. A fixture's path is the same in each run (`target/fixtures/x<file's
hash>/<name>_<n>`), so the same test compiles the same thing there.

- **What it saves:** a run where the compiler's build is the same, a
  change to the tests or docs alone, or `check:fast` again: on the 12-core
  M6 1 min 47–50 s, not 2 min 56 s. After any change to `src/`, the
  runtime's helpers among it, nothing's found: about as before, 2 min 59 s
  to 3 min 23 s.
- **What it never keeps:** Cargo's builds (`rust-js build`, the cargo
  tests), a compile that fails (207 a run, 12 s summed: not worth a
  record, #886), one that read a file it wrote (a manifest
  of the build before), one of a crate with hooks or `--declarations`,
  a mutant's, the determinism net's (`RUST_JS_COMPILE_CACHE_MB=0` in its
  environment), and a script standing in for the compiler.
- **Bounded:** 2 GB, or `RUST_JS_COMPILE_CACHE_MB` (0 keeps none). Each
  use marks its entry; past the cap the least recently used go first,
  checked as a process's writes would pass it and as each `check:fast` or
  `bun run test` starts, which also drops every entry of another build,
  never found again. Below the disk's 40 GB floor nothing's written, said
  once a process, and what's kept is pruned until the floor's back or
  it's empty. `cargo clean` removes it with the rest of `target/`; `rm -rf
  target/compile-cache` is always safe.
- **If you doubt it:** `RUST_JS_COMPILE_CACHE_MB=0 bun test <file>` runs
  every compile; CI has no cache, so it's the check the cache changed
  nothing.

### What the loop keeps on disk, each bounded

Every cache is bounded: a cap, the least recently used going first past
it, what's stale pruned, nothing kept below the disk's 40 GB floor, a test
that holds it, and its cap here. `test/bounded.ts` is the loop's floor, and
`tooling/bounded.js` the one way a store is held to its cap, an app's
cache's too (#1057); each store says what its things are, and its floor.

Everything development writes has one known place (#1213, ADR 1118):

- **`.cache/`** in the main checkout, gitignored, which every worktree
  finds through the git common dir: the machine's stores below.
- **Each worktree's `target/`**: its build and its stores below.
- **The temp root**, `$TMPDIR/rust-js-dev/`: every test process's
  `TMPDIR` (`test/setup.ts`), so what a test or a tool it runs leaves there,
  Vitest's module cache of each run, is held too. Outside the checkout, as
  a fixture made there is an app outside any checkout of rust-js.
- **The tools' own**, outside: rustup's toolchains, Cargo's downloads,
  Bun's install cache, Playwright's browsers. Each tool's home is set by
  the tool, `CARGO_HOME` and `RUSTUP_HOME` from the environment alone, so
  a plain `cargo` in the checkout would use `~` whatever the repo set;
  they're held by their own commands instead.

`bun run disk` lists every store, its size, cap and oldest entry, and the
total inside the checkouts against its 40 GiB budget; `bun run disk
--prune` holds each to its cap, the tools' by `rustup toolchain uninstall`
of an old pin, Cargo's GC and `bun pm cache rm`. `check:fast` prunes the
stores inside the checkouts as it starts and warns past 90% of the budget.
`scripts/disk.ts` is their catalogue, and `test/disk-owners.test.ts` holds
that no other script makes a path in the user's home.

| What | Where | Bound | Held by |
|---|---|---|---|
| The tests' compiles to JS (above) | `target/compile-cache/` | 2 GB; another build's dropped as a run starts; nothing written below the floor | `test/compiles.test.ts` |
| Native programs' builds | `.cache/native-cache/`, the machine's, every worktree's and whatever `RUST_JS_TARGET` a mutant's or the test map's tests run in (#875, #1047), keyed by what they're built from said from the checkout's root | 10 GB, the corpus's and the metamorphic net's about 8 GB, and a week unused; nothing kept below the floor, a build there used from a fixture; pruned as each `check:fast` (while the compiler builds) and `bun run test` starts, and below the floor until it's back | `test/support.test.ts` |
| Native programs' wrappers | `target/native-sources/`, the checkout's likewise | 64 MB, and a week unused, the same way: about 8 MB | `test/support.test.ts` |
| A corpus case's and a metamorphic rewrite's verified runs (ADR 0567) | `.cache/verified-runs/`, the machine's (#1047); a mutant's tests', their target's `corpus-cache/` | 512 MB, a week unused, the same way: about 8 MB a build of the compiler of the corpus, 100 MB of the net | `test/support.test.ts` |
| A test's fixtures | `target/fixtures/`, `$TMPDIR/rust-js-dev/rust-js-fixtures/` | Removed as its file's tests end (`test/setup.ts`'s `afterAll`; Bun's test runner never emits `exit`) or a script exits; one a killed process left, by the next process's first claim. `RUST_JS_KEEP_FIXTURES=1` keeps a run's to look at | `test/support.test.ts` |
| The apps' Cargo targets' seed, the binding crates checked (#1005) | `target/check-fast/<pid>/apps-seed/`, `check:fast`'s run's; else a fixture of the file that asks | One a run, about 330 MB, 260 of it rustc's incremental state, which the targets made of it don't take; gone as `check:fast` exits, or with the file's fixtures; one a killed run left, by the next `check:fast` | `test/support.test.ts` |
| The crate corpus's Cargo projects (#1213) | `target/crate-corpus/<crate>/` | 2 GiB, the least recently measured crate first: 600 MB from nothing, each build of the compiler adding a copy of every crate's dependencies (25 GB before) | `test/support.test.ts` |
| The mutations' worktree and builds | `target/mutants/` | 4 GiB, its entries the least recently changed first | `test/support.test.ts` (`boundEntries`) |
| The temp root (#1213) | `$TMPDIR/rust-js-dev/` | 2 GiB, its entries the least recently changed first: Vitest's `<id>/ssr/` of each run, which it never removes (740 of them, 1.5 GB, before), tests' `mkdtemp`s | `test/support.test.ts` (`boundEntries`) |
| The guide's examples, `bun run docs` | `target/docs/` | Each removed once its output's read, as each run makes it anew; an app's Cargo build there is 290 MB | |
| The test map's maps (ADR 1101) | `.cache/test-maps/`, the machine's, which every worktree reads (#932) | 3, about 12 MB each, the least recently used going first, each touched as a check reads it; one of another toolchain or layout removed as a recording ends; none kept below the floor | `test/test-map.test.ts` |
| React's port, `bun scripts/react-in-rust.ts` | `$TMPDIR/react-in-rust-<checkout's hash>/` | 4 GB together; one whose checkout is gone removed as any run starts; then, of those no run used in the last hour, the least recently used past the cap or below the floor, never the run's own. Its build is of one build of the compiler, another's removed, and without rustc's incremental cache: about 215 MB after a regen | `test/support.test.ts` |
| An app's cache, `rust-js build`'s, the examples' and every user's (#1057) | `<app>/node_modules/.cache/rust-js/`: a Cargo target for each of `target`, `editor`, `clippy` and `test` | 2 GB together, the least recently used target first, never the one in use, and none but it kept below 5 GB free; a target another build of the compiler made emptied as a build starts, but what its builds wrote in source (`holdTarget`); about 500 MB the Vite example's. `CARGO_TARGET_DIR`'s, the user's, isn't held | `test/in-source.test.ts` |

One `check:fast --all` from an empty `target/` on the 12-core M6 used at
most 5.1 GB of the disk and left 1.6 GB, the native programs and the crate
corpus's builds (before #914: 12.3 GB, all of it left, 7.3 GB of fixtures
and 2.6 GB of the guide's apps). A regen of React's port leaves 215 MB, the
same after each change to the compiler (before: 640 MB, and 530 MB more
after each).

What isn't a cache, and goes with `cargo clean` or by hand: `target/debug`
(about 2.5 GB), `target/mutants` (delete it after each mutation run),
`target/crate-corpus` (0.4 GB, its crates' builds, kept for its next run),
and `target/test-map/build` (1.1 GB, the one counting build of the
compiler the test map records with, built again where the next recording
needs it; a recording's own counts and targets go as it ends). The maps
themselves are the machine's, outside `target/`, so `cargo clean` keeps
them.

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
| `bun run check:fast`, of a change to the lowering | 40 s; on the 12-core M6, 53–60 s of a statement added to tidy, drops or the bindings, 74 s to an iterator's code with the rows it names, with the test map (2026-11-08, ADR 1101) |
| `bun run check:fast --all`, what a push's check runs that can run here (195 files and the corpus at once, then the browsers' 6) | 2 min 36 s before the browsers' files and `cargo test` (load 12–16, 2026-11-04); 6 min at a load of 20–33, its corpus parts and two other jobs killed at 290 s, each passing alone after: the other files in 116 s, the corpus in six parts in 239 s (2026-11-05) |
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
  changes; CI runs the rest. **`--all` runs what a push's check runs**
  that can run here, so the check before a push is it and the change's
  mutations, no list beside it: typecheck, fmt and clippy, `cargo test`
  (its own timeout, 600 s, as its build waits on clippy's lock of
  `target/`), the Vite example's build, and every test file of the suite
  (`scripts/test.ts`), one `bun test --parallel` of them beside the
  corpus's parts, then the browsers' (`scripts/shard.ts`'s `browsers`) on
  their own, as the suite runs them, as beside the corpus they starve; the
  nightly's own (`nightly`) are left as the push's suite leaves them, tidy's
  stages run where tidy changes. The list of suites a `src/` change could
  break left out rest-parameters' and cross-crate-items', and `main` went red
  three times in a day (#607); `cargo test` and the browsers' files, left to
  CI, went red on `main` unseen, `finish_is_the_last_stage_in_its_order`
  after 801d40c55 and the pilot's committed JS at 336984bff (#732).
  `test/ci.test.ts` holds it to `check.yml`: each command a job of the
  check runs is a job's here, or one `leftToCi` names with why (the WASM
  compiler, rustc's whole suite, all of a check's mutations, a machine's
  setup), and each test file its suite runs is one a job here runs. Its 195
  files take 2 min 35 s beside the corpus under another agent's load
  (2026-11-04). Each job
  is given the compiler it built, `RUST_JS_COMPILER`: one that builds its
  own links it again, and races the others.
- **A job's timeout is scaled by the machine's load** (`loadScale` in
  `scripts/timing.ts`): its usual timeout, 290 s for most, times the
  highest load average over the cores since its phase began, from 1 to 3,
  as the run's own jobs raise it (a load of 7 as one began was 53 a minute
  on). A test's under Bun is too, its minute's default (`test/setup.ts`)
  and each one a test writes of three minutes or less, about 90 of them,
  `}, underLoad(60_000));` (`test/ci.test.ts` refuses one written alone):
  at a load of 43, inbox's snapshot passed 120 s and a tidy stage's test
  its 60 s, each a few seconds alone. With six or seven agents at once the M3 Max sat at a
  load of 20 to 67 on its 12 cores, and a fixed 290 s stopped every corpus
  part, the other files and the readability passes in most runs, each of
  which passed alone afterwards, the corpus in six parts in 239 to 341 s
  (2026-11-05). Each agent then ran them again by hand, which cost more
  than stopping them saved and lost the timings `check:fast` keeps. A hang
  still ends, at three times its usual timeout at most, and the run says
  the scale it used.
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

- **A workspace is one `rust-js build`** (ADR 1068): every member at its
  root, or each `--package` named, in one `cargo check`, each crate checked
  once and the crates that don't use each other on every core. Which
  crates are programs is the workspace's, the same for every build, so a
  crate one build checked is done for the next. React's port, `bun
  scripts/react-in-rust.ts regen`, on the M3 Max under another job's load
  (2026-11-02): 475 s cold, 931 s of CPU, up to seven crates at once, and
  1.7 s warm; one `--package` build per crate took 2,186 s cold and
  2,405 s warm, one core busy, each dependency checked again in every
  build that used it. On the M1 Mac mini (2026-11-05, load near 3): about
  1,110 s cold, its last 8 minutes `react-reconciler` alone on one core,
  which the crates after it wait for. Since ADR 1100 (2026-11-07), each
  crate's compile answers each question once: on the 12-core M6, 75 s
  after a change to `src/`, not 504 s, the reconciler 23 s.
- **A corpus case's runs are kept by what they ran** (ADR 0567), in
  the machine's `.cache/verified-runs`, as the metamorphic net's
  are (#1047): the same JS, native output, runtime modules and
  harness again, its four runs aren't run again; its two compiles always
  are, and its snapshot is always checked. A `check:fast` whose change
  leaves most cases' JS as it was runs the corpus in half the time.
  What no test used for a week is pruned by `bun run test`, and by
  `check:fast` once a day (`target/kept-pruned`), as only CI runs the
  whole suite: a Mac's `target/native-cache` grew 23 GB in two days.
- **A push's corpus builds as libraries only the cases marked `//@
  library-differs`** (ADR 0360, amended), about a tenth of them, as
  `check:fast`'s does (`RUST_JS_LIBRARIES=differing`); the nightly, a
  mutation's run and a plain `bun test test/corpus.test.ts` build every
  case, and say where a mark is missing or stale, which `bun run bless`
  fixes. The 1,319 cases on the M3 Max: 26 s, against 47 s building all.
- **The react, webapi and js crates' metadata** is built only when their
  sources, build scripts or the toolchain change: a stamp beside it,
  `target/libreact.stamp`, says what it was made from. Built every run, it
  was three seconds of a focused test's four.
- **rust-js is built with one `RUSTC_BOOTSTRAP`**, `.cargo/config.toml`'s,
  by the tests as by a shell. `.env.test` gives the tests' programs `1`,
  and a build with it rebuilt every dependency, 22 s, and the next one
  without it again.
- **`@rust-js/std` is generated**: `bun run std` after a change to a
  helper or to what std's bodies lower to, then a rebuild, which
  builds its manifest in; `test/std-package.test.ts` holds it (ADR 0561).
  Its `core` is built to unwind, of the toolchain's `rust-src`, once for
  each toolchain and how it's built, 2.4 s, kept in `target/debug/std-sysroot`
  (`src/std_sysroot.rs`, #424).
- **Mutations keep their crate** in `target/mutants/crate`, writing only
  the files that differ, and the compiler built without a mutation while
  the sources are what it was built from. They run up to four at a time,
  one worker for each two cores (on this Mac 1 took 70 s for sixteen, 2
  took 43 s and 4 took 31 s; CI's four cores run two), and no more than the
  memory holds, 3.5 GB a build beside 0.3 GB a test on each core (two on 8
  cores and 16 GB, `compilerBuilds`, #1084), each with a crate
  and a target of its own (`--jobs=N` says how many). Each one's build and
  tests are timed: CI's summary shows them, and each shard uploads
  `mutation-timings-<shard>`, each a share of its build. A runtime helper's
  mutation has its own runtime too, installed in its tests' target
  (`node_modules/@rust-js/` there, `test/packages-of.ts`), which the JS its
  tests write imports. So its tests run beside the others', and the
  checkout's `std/runtime.js` is never rewritten (#720).
- **Mutations are built up to 32 to a compiler** (ADR 1031), each chosen
  by the name of its copy of it: on CI, where one built alone took 28 s, a
  build of many takes about as long as one. `--batch=1` builds each alone.
  A build of many that fails is built again as two halves, so a mutation
  that doesn't build alone says so; one whose build wouldn't lex isn't
  built, and is named as `doesn't lex`, which `test/mutations.test.ts`
  checks of every mutation in the check (#479). The controls run each file the
  mutations name once as the compiler is and once broken, read from its
  JUnit report; a build's mutants test on every core (`--tests=N`) while
  the next builds; and each mutant's run stops at its first failure
  (`bun test --bail`), which is the answer (ADR 1031, amended).

## Keeping the loop fast

The loop is part of the product: a slow step is paid on every change, by
every agent at once. Noticing one and making it faster is everyone's job,
as a wrong answer is.


**Measure each task's time, and how much of it went to checks.** The
loop's speed-ups are worth what they save agents, so each task reports it:
`bun run task start <name>` when it starts; every check or test through
`bun run task run <label> -- <command>` (check:fast, the corpus, mutations, a
port's regen, a matrix run); and `bun run task report` at the end, before
`cargo clean`. The report's wall time, check time and its share go in the
hand-back, and on the task's issue. The log is per branch in
`.cache/tasks/`, so `cargo clean` keeps it.

**Look for speed-ups as you go; it's part of every task.** Issues about the loop carry the `dev-loop` label (`gh issue list --label dev-loop --state all`): it's the list of what was tried, what worked and what didn't, with numbers. Each one
speeds up every agent on every machine from then on. If a step makes you
wait, notice it: time it, find out why, and fix it in the same task when
the fix is small. Otherwise list it as a slow spot below. When you speed
something up, or try something that doesn't pay, add it to [the loop's ledger](docs/dev-loop.md)
with its before and after, so the next agent sees what kind of fix pays.

The ledger of every speed-up tried, with its numbers, its status (active, dropped or superseded) and its commit, is [docs/dev-loop.md](docs/dev-loop.md). Add to it there, never here.

- **Record the test map when a task starts**: `bun run test-map` after
  `git reset --hard origin/main`. The maps are the machine's, kept in
  `.cache/test-maps/` for every worktree (#932): where any
  worktree recorded the commit's, it ends at once; otherwise about 150 s on
  the 12-core M6 (gate it as a heavy step), and each other worktree of that
  commit, or of one after it, reads it. Plain `check:fast` then runs only the suites' files and
  the corpus's cases that ran a function the change touches, of an
  iterator change the cases built with std's iterator structs too, and
  says what it picked or why it can't say (`scripts/testMap.ts`, ADR 1101):
  an iterator's change ran in 74 s, not 153 s. It can say while all that's
  changed since the map's commit is a function's code in `src/`, a plain
  suite's test file or a corpus case; after committing a doc, a test's
  helper or anything else outside `src/`, record it again, or the check
  runs what the change's paths say, as without a map. `bun run test-map
  pick` says what the working tree's change picks; `--keep` keeps a
  recording's counts, to see why a test runs on every change. `--all` never
  reads it.

- **Every step runs on all the machine's cores.** The Macs here have 8 to
  12 cores, and an agent's time is mostly waiting on a step, so a step
  that runs on one core while the rest sit idle is wasted development time.
  Every way of running work here has a parallel form; use it:
  - the corpus and any file of cases: in parts, `RUST_JS_CASES=i/n` each,
    as `check:fast` runs them (`caseParts`), with `BLESS=1` per part when
    blessing, never one process of the whole file;
  - test files: `bun test --parallel`, or `check:fast`, which runs them at
    once;
  - mutations: `--jobs=N` workers and `--tests=N` (`scripts/mutations.ts`);
  - builds: a workspace whole, with `--package` as often as needed (below);
  - your own scripts and investigations, such as a bisect, a probe over many
    cases or a sweep of seeds: in parallel processes (`xargs -P`, a `Promise.all` of
    `Bun.spawn`s), never a loop of one at a time.
  
  Size the parts to the machine's cores; with one agent per machine (below)
  they're all yours. Where work still runs on one core
  and could be split, split it: give the tool a parallel form, or add it to
  the slow spots below with its cost and what would split it. A step is
  done when it uses the machine, not only when it passes.
- **Use the loop's own tools, so its timings see you.** `bun run
  check:fast` is the change's checks at once, timed against their usual,
  and says how busy it kept the cores; `--profile` says it of each minute,
  with each job's start, end and cores, to find where they sat idle on any
  machine;
  hand-rolled runs (a corpus shard in the background, a build then a test)
  are timed by nobody, and their slowness goes unseen.
- **Read a `⚠ slower than usual` line before going on**: find what made
  it slower, fix it or write it in the list below.
- **When a step is slower than it needs to be**, fix it if it's yours to
  fix: a cache, a stamp, a parallel run, a smaller test. Otherwise add it
  to the list below, with what it costs and what would fix it, so the next
  agent starts from it. A fix moves its line to "What keeps them there",
  above, or out.
- **Build a workspace whole**: `rust-js build` at its root, or with
  `--package` as often as needed, never a loop of one build per crate,
  which runs one core at a time (ADR 1068).
- **Build what the tests use.** The tests, `check:fast` and an app built
  from this checkout run the debug compiler, `target/debug/rust-js`
  (`bun run build`); `cargo build --release` builds one nothing here runs.
  That build is optimized, `opt-level = 1` without LTO (Cargo.toml's
  `[profile.dev]`, ADR 1100, amended, #896): it compiles three times as
  fast as unoptimized, and an edit still builds in about 2.5 s. One profile
  for every build of the compiler the loop runs, CI's too; don't set
  another with `CARGO_PROFILE_DEV_*` for one step, which builds a second
  compiler. A mutation run's first build, of `target/mutants` from
  nothing, takes about 29 s, not 17 s.
- **One agent per machine, one task at a time** (the user's rule,
  2026-11-05). One agent running its corpus, mutations and builds on every
  core beats several sharing them. Eight agents on one 12-core, 16 GB Mac
  ran at a load of 50–60, each step three to five times slower, with
  timeouts and reruns; four or five at a time made each two or three times
  slower. A session doesn't spawn agents to work in parallel: it finishes
  one issue, then starts the next. With three machines, three issues are
  in flight.
- **Fill the gaps between heavy steps.** With one agent, the machine sits
  mostly idle while it reads, edits and runs one focused test (64% idle
  measured beside a 12-core agent, 93% beside an 8-core one, 2026-11-06).
  So run what's independent at once: both corpus builds (the default and
  `RUST_JS_LIBRARIES=all`) as one set of parts, not one after the other
  (about 3 of 7 minutes saved on 8 cores); a bisect's or a probe's runs as
  parallel processes; the next build while the last results are read.
- **Mutations of a change: `--changed=origin/main`**, never
  `--changed=main`. A worktree's local `main` can be far behind; one such
  run picked up 1,337 files and was filling the disk. `--list` says which
  it chooses, each name and file, in a second, and runs none; a flag the
  runner doesn't know is refused (#923). A run's last line names the
  mutations not caught, so a `tail` of it is enough.
- **A gate or script that takes a lock releases it on any exit**, a
  signal included (`trap`). A stopped gate once left its mutations lock
  behind, and the next run waited on it overnight.
- **Keep the Mac awake** while work runs unattended (`caffeinate -dims` in
  a terminal): a sleeping Mac pauses every process, and `timeout` with
  them, so a five-minute command took hours and its tests failed on time.
- **Never search the whole disk** (`find /`): it ran sixteen minutes and
  slowed every agent. A dependency's source is where Cargo keeps it: a
  crates.io one under `~/.cargo/registry/src/*/<name>-<version>`, a git
  one, as oxc is, under `~/.cargo/git/checkouts/<repo>-*/<rev>/`.

### When the machine stalls

Every stall below was seen on these Macs. Before you believe a timeout or a
slow step, check `uptime`, `memory_pressure`, `df -h ~`, and
`ps -Ao pid,ppid,%cpu,rss,etime,command -r | head`. Then compare the
symptom with this table. When you hit a new kind of stall, add a row.

| Symptom | Cause | What to do |
|---|---|---|
| A 5-minute command takes hours; `timeout` never fires; tests fail on their 10 s limits | The Mac slept. `timeout`'s clock stops in sleep too (`pmset -g log \| grep -E " (Sleep\|Wake\|DarkWake) "`) | Keep it awake with a detached `nohup caffeinate -dimsu & disown`, which outlives a session restart. Check `pgrep -fl caffeinate` after any restart. On a laptop, closing the lid sleeps anyway. |
| New binaries start 300 ms late; corpus and native checks run 3–5× slow; `XprotectService` and `syspolicyd` are busy | macOS's scan is on for the app the commands run under, often after an update of that app | See [The Mac, with the scan off](#the-mac-with-the-scan-off): find the app just below `launchd`, add it, restart it, and time 20 first runs. |
| Load 50–60 on 12 cores; every step 3–5× slower; timeouts and reruns | Too many agents on one machine (8 worktrees at once) | One agent per machine. |
| Swap grows; builds slow; memory pressure warns; Claude Code stops a background command "for low memory" | Several builds of the compiler at once on 16 GB. Each peaks at 3.4 GB building the crate whole (a mutation target's first build, or a batch's of many files), 1.6 GB after one file's change, `cargo test`'s 2.0 and clippy's 1.8; a test file or a part of the corpus or of a net 0.1–0.3 GB (the largest 1.8), the crate corpus 2.1 GB, `check:fast --all` whole 5.0 GB (measured on 8 cores and 16 GB, 2026-11-09). Mutations' four builds at once beside a test on each core stopped a gate there (#1084) | Runs are sized to the memory as well as the cores by one owner, `scripts/shard.ts`'s `peakGB`, `memoryBudget` (the memory less 6 GB, and what's free as the run starts less 2 GB) and `compilerBuilds`: mutations' `--jobs` and check:fast's pool ask it. A runner of your own that builds the compiler more than once at a time asks `compilerBuilds` too. |
| A command can't start: `ENOSPC`; agents stop mid-commit | The disk filled: worktrees' `target/` (10–20 GB each, `target/mutants` above all), a `--changed=main` mutation run of 1,337 files | Keep 40 GB free. Delete `target/mutants` after each run, run `cargo clean` when a task ends, and remove finished worktrees. Use `--changed=origin/main`. |
| A gate waits forever before it starts | A stopped run left its lock (the mutations lock) behind | Release locks on any exit with `trap`, and check for a stale lock before waiting on it. |
| Load stays high with no agent working | Orphans: a finished or restarted session's `bun`, `node`, `rust-js` or `cargo` left running (`ppid` 1) | List them with `ps -Ao pid,ppid,etime,command \| awk '$2==1'`, check their worktree (`lsof -p <pid> -d cwd`), and kill only those of finished work. |
| "main is red" after a rebase, and nobody else sees it | A build or regeneration ran while the rebase was stopped on a conflict | Wait until `git status` no longer says `rebasing`, then rebuild. `check:fast` refuses to start while a rebase is stopped (`git rev-parse --git-path rebase-merge`); a gate script of your own should too. |
| Everything slows for minutes | A whole-disk search (`find /`) | Search only where Cargo keeps sources (see above). |
| A test file times out at its deadline only under load (`test/tidy-stages`, 109 s alone) | Its cases compiled one after another inside one test | Compile each case's variants at once, and share one folder per case (#808: 109 s became 31–38 s). |
| A mutation run takes hours | An empty name list, often from zsh not splitting `$VAR` into words, runs every mutation (8,000+) | Read names from a file, and stop with an error when the list is empty. |
| A sweep's results can't be trusted | A shared input, such as `std/runtime.js` or the compiler, was swapped while the sweep ran against it | Never change what a running sweep reads. Probe in a copy of the worktree, or wait for the sweep to finish, and rerun it if it overlapped. |
| `test/in-source.test.ts`'s "the builder asks the compiler what it is" times out at 120 s in `check:fast --all`, the cores half idle; alone it passes in 0.2 s (unwindC, 2026-11-08, once in five runs) | Not found. It runs the builder in a fixture, which runs Cargo, so a lock Cargo waits on (its package cache's, or `target/`'s) is suspected | Rerun the file alone; if it hangs again, `ps` for a `cargo` it started and what it waits on, and add what you find here. |
| A session restart, and all its agents stop | The host app was restarted or updated; agents' background jobs die with it | Commit WIP often. After a restart, check each worktree, clear stale gate entries and locks, and resume each agent with its hand-off. |

**Good practice:**
- One agent per machine, with every heavy step at full width.
- Independent steps at once: both corpus builds, the checks after a build, and probes with `xargs -P`.
- Commit WIP before long runs.
- `cargo clean` and worktree removal when a task ends.

**Bad practice:**
- Several agents sharing one machine.
- Running a suite as one process when it has parts.
- `--changed=main`, or a lock without a `trap`.
- Building mid-rebase, or retrying a timed-out command with more time instead of finding out whether it hung.

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
- **A matrix row the change names fails, and the change can't explain
  it:** build `main` in a worktree of its own and run the row's batch
  there, narrowed to the fewest rows that keep it wrong (the gate's rows
  are rebatched, so a row whose JS depends on its neighbours can fail that
  the full run's batch hides, #850). Failing on `main` too, it's a wrong
  answer of its own: file or fix it first, then land.
- **Regenerate what's generated on a conflict**, never merge it by hand:
  `docs/readability.txt` with `BLESS=1 bun test test/readability.test.ts`
  (its shares are compared over the files it lists, so a case added is no
  reason to bless: it's counted once anything else is blessed),
  corpus snapshots with `bun run bless`; the decision index merges by
  union, and the architecture test names a line left twice.
- **After a rebase, check again only what the incoming commits touch**.
  With several agents pushing, `main` moves about as often as a
  `check:fast --all` takes (about 10 minutes under load), so a full re-run
  for every commit that landed meanwhile is a loop that never ends. Once
  your change's whole check has passed:
  - If an incoming commit touches the code your change works with, run the
    whole check again. For an iterator change that means any of
    `src/lower/mir/iter`, `src/lower/iterators`,
    `src/lower/analysis/back_stepped.rs`, `src/runtime`, `std/`,
    `test/corpus` or `test/iterator-matrix-known.txt`.
  - Otherwise, and when the rebase applied with no conflict: rebuild and
    regenerate (`cargo build`, `bun run std`), then run
    both iterator slices (`bun scripts/iteratorSlice.ts run matrix` and
    `run conformance`) and the corpus cases your change touches, and push.
  - Never build or regenerate while a rebase is stopped on a conflict: a
    binary or bundle built from a half-applied tree fails, or passes, for
    no reason of its own. That once sent several agents after a red `main`
    that wasn't.
- **Commit what the change regenerates**: `std/` is committed, so a change to `src/runtime` or `std`'s sources commits
  them regenerated. Reset neither to `main`'s copy to tidy a diff: the
  runtime-package test fails on `main` once the committed bundle lags the
  compiler, and every other agent's check fails with it.
- **After a conflicted merge of `scripts/mutations/*` lists**, typecheck
  and run every mutation of each conflicted list, both sides'. A cut
  between two entries once merged one into the other (typecheck's
  duplicate keys), and another moved a `snapshots: true` onto a new
  entry, so its mutation survived on `main`.
- **Removing code because its mutation survived** (the code changes no
  answer) needs more than the corpus and the sampled iterator slices: run
  the full matrix rows of every adapter the removed code decides for
  (`allCases().filter(c => /enumerate/.test(c.id))` given to `program()`,
  about 30 s). A rule removed on the corpus and slices alone broke 19
  matrix rows (#823). The same goes for a generated drop-order sweep where
  the code is about drops.
- **Land a big change's prerequisites first, each as its own commit.** A
  precision fix or a new rule that improves the output without the big
  change can land with a small gate of its own. #847, an abort on a panic
  during unwind, found three blockers one after another: a precision fix,
  React's labels, which an early-return rule fixed, and 32 mutations no
  longer caught. Each blocker cost another 30–45 minute round of checks,
  over four agents' hand-offs. Landed first, the precision fix and the
  early-return rule would have been on `main` already, and the flip would
  have carried far fewer surprises.
- **A form-only mutation of tidy, preparation or the printer is judged by
  the corpus's snapshots, by the runner**: where its tests pass, the cases
  that reach its site (by the test map, or every case without one) are
  compiled by the mutant and each JS compared with its snapshot alone; one
  that differs catches it (#864, ADR 0093 amended). Name the case it's for,
  as with any mutation; no lowering test of its form is needed. Of other
  code, the runner judges the corpus by what the JS does alone
  (`RUST_JS_SNAPSHOTS=ignore`), so a form-only mutation there still needs a
  lowering test or `snapshots: true` on the cases it names.
- **ADR numbers**: `bun run adr:next`, and `bun run adr:renumber` when
  `main` took yours.

### Slow spots, still to fix

| Step | What it costs | What would fix it |
|---|---|---|
| `check:fast --all` of a change outside the iterators, bound by its CPU (#986, #996) | On the 12-core M6, quiet (load 1.42), warm with no change, 126 s and 1,160 job core-seconds after #882's round of 2026-11-09 (146 s and 1,219 before), the most the apps' Cargo builds: crates 68, docs 61, the crate corpus 57, next 53, pilot 51, cargo-workspace 46, libraries 46; before that, quiet (load 1.33), a compiler change: 128 s, the cores 96–97% busy each minute, some 1,475 core-seconds the machine's, about 1,295 the jobs' (the 208 test files about 780, the browsers' about 200, the crate corpus 80, the readability passes 36): a floor near 120 s on 12 cores. Each test file is a job of the pool already, the longest first; the four longest, libraries, vite, next and cargo-workspace, end at 1:29–1:32, before the files queued behind them, so splitting them saves 5–8 s at most | Less CPU in the suite. #998 took the readability passes off a change that isn't tidy's or preparation's (123 s, load 2.41, 1,251 core-seconds), kept the crate corpus in every `--all` (the user), and found the heaviest job, `test/docs.test.ts` at 117–119 core-seconds, and the apps' Cargo builds behind it and next, vite, crates, libraries and cargo-workspace: #892, each app's binding crates built under a key of its own and a Node session for each `-vV` |
| Plain `check:fast` where the test map can't say (ADR 1101) | After any change outside a function's code in `src/` since the map's commit, a doc, a test's helper, a type or a constant, it runs what the paths say: 53–144 s of the five changes measured, not 56–121 s; recording again is about 130 s. And a change to a function every compile runs, tidy's or the runtime's `source`, picks nearly every file and case (101 of 101, 1,805 of 1,809), so the check is CPU-bound on all of it | A helper's change picks its importers (a test's text reads of the checkout now pick it, `readersOf`, 2026-11-09); and a map recorded once a push, not once a commit on each machine (#932 shares a worktree's with the machine's others; CI's runners are Linux's, whose `cfg`s compile other functions, so its map isn't the Mac's) |
| `check:fast --all`, now bound by its CPU, not by its cores idle (2026-11-07) | On the 12-core M6, about 1,800 CPU-seconds with no change, 900 of them the 200 files besides the corpus (the longest `crates`, `libraries`, `snapshots`, `cargo-workspace` and `compiler`, 45–80 s each), 350 the corpus's compiles, whose runs are cached; with an iterator change about 3,500, 1,270 of them the rows it names and the corpus with std's iterator structs. Its cores 90% busy, half of them efficiency cores. Measured by #874: rust-js's processes are 1,780 of its CPU-seconds, 9,050 calls: 380 the corpus's, 350 the fixtures' compiles, 260 the rest, and Cargo's builds of crates, which #885 measured again apart from the rustc each one runs: about 590, of which the binding crates are 140 (`webapi` 114 s, built 31 times, at 3.4–4.1 s each under load, 1.8 s alone; `react` and `js` 11 s each), the `-vV` probes 75 (about 590 a run, 0.13 s each, every one a Node session that asks which Prettier the app has), and the rest the crates the tests and the crate corpus compile. The file compiles are kept now, where the build is the same (1 min 39–50 s with no change); after a change to `src/` none is (3 min 4 s) | Less work, not more parallel. Not native Rust's outputs kept, under 2% of any job's CPU (dropped, docs/dev-loop.md, #875). Not the binding crates in a target the tests share, which can't share them: Cargo's `-vV` line names each app's crate directories, by which Prettier they find (ADR 1003), so `webapi`'s `-C metadata` differs in every app in the checkout, 31 keys for 31 builds (dropped, docs/dev-loop.md, #885; what would let them share, #892) |
| Every mutation on the Mac, `bun run nightly mutations` | 65 min for 8,047 on the M3 Max (2026-10-30), the CPU 86% busy and its 16 GB full: 6.6 GB compressed, 1.3 GB swapped, 35% of the CPU the system's, each mutant's run starting bun, rust-js and Node | More memory, or a mutant's tests run in a process kept warm, as `bun test` with its compiler swapped, not started again for each |
| A mutant's serde, shipped-crates and bound-maps tests (145 mutations) | Each builds its native program again in the mutant's own target, its wrapper `include!`ing a file beside it in a fixture there, so the path, and the kept build's key, is the mutant's; a serde build links serde's rlibs. The corpus's wrappers are kept by content in the checkout's target (#875) | The wrapper and the files it includes written to a `contentDirectory` of the checkout's, as `runNative` writes the corpus's |
| `bun scripts/mutations.ts --changed=origin/main` of a large change | More than five minutes even in three shards, so it's left to CI | Shards picked to fit, by each mutation's usual time |
| Two or more agents' worktrees | A full `cargo build` and corpus each | A shared build cache across worktrees |
| A fresh worktree's builds of the compiler, `cargo build` and the test map's counting build where no worktree recorded the commit's map | 43 s and 44–46 s on the 12-core M6 (2026-11-08, #932): the compiler's own crate 27 s, incremental, which sccache never keeps, and of its 146 dependencies the 56 that are or are built with a proc macro, whose dylibs differ by target, so sccache misses them in every new target, the critical path's 17–19 s. Since #932 a map is recorded once a commit on the machine, not once a worktree | Proc macros' dylibs the same in every target, or the dependencies built once for the machine; a target the worktrees share isn't sound for the compiler's own crate, whose freshness Cargo judges by its files' times (ADR 1101, amended) |
| `check:fast --all`'s readability passes, `test/tidy-stages.test.ts` and `test/getter-reads.test.ts` (ADR 1087) | 4 min 17 s beside the corpus at a load near 13 (2026-11-04). Its control test, the tidy stages' first against their last, compiled `interpreter.rs` once for each rewrite that fired, one after another, again after its case did: 12 s alone, past its minute in every loaded `check:fast --all` (2026-11-06, #808). Now each case's variants compile at once, on every core, and the control test reads the cases' compiles: on the M1, 109 s alone to 31–38 s, and beside the corpus 135 s to 51 s, the control test 15 s to none, `interpreter` 51 s to 20 s; in `check:fast --all`, its job 10 min and failing to 3 min 55 s passing, at a load reaching 32 on 8 cores | Its nine cases' 16 tidy stages each still compiled apart: one compile writing every stage's JS, as `--prepare-fired` reports every rewrite in one |
| Clippy's crates in the compiler, for `rust-js clippy` (ADR 0888) | A cold `cargo build` builds them first: 12 s (29 s of CPU) of its 59 s on the M3 Max, and 330 MB more of `target/`; the debug binary is 104 MB, not 59 MB. A build after an edit is as before, 3.5 s | Built once for every worktree, the shared build cache above |
| `check:fast`'s bindings job while other worktrees build | Failed once at 35 s and passed alone, 75 of 75 | Find what it races on |
| macOS's `xprotectd` scanning freshly built binaries and fixtures | Load near 20; every corpus case and suite timed out at about 270 s against 30–50 s, passing once it settled | The scan off for the apps that build ([The Mac, with the scan off](#the-mac-with-the-scan-off)); check `ps` for `xprotectd` before believing timeouts |
| A sweep of generated programs against native Rust, by hand | 4–5 min for 1,100–2,100 programs, again after each fix, each a `rustc`, a rust-js compile and a run | A checked-in runner: native outputs kept by source, only rust-js's compile and run again, verdicts said against the last sweep (fixed, regressed) |
| `test/architecture.test.ts`'s "a binding's optional argument is an Option of its one function" under load | Times out at bun's 5 s in some loaded `check:fast` runs (5.2 s), a re-run each time | A timeout that fits its compile |
| A compile of std's bodies, `bun run std` and `test/std-package.test.ts` | 8.5 s alone on the M3 Max (6.2 s before `Filter` and the adapters' `try_rfold`, ADRs 0930, 0932; 5.4 s before `StepBy`; 1.3 s before the iterator plan, ADR 0908), past 60 s beside `check:fast`'s other jobs, so its timeout is 120 s; each mutation of a std body's lowering compiles it again | The debug compiler is the cost: a release build of the compiler for the std build, or the std package compiled once per compiler and reused by the tests that only read it |
| A mutation that makes the compiler never finish, `drop-of-its-own-type-inlined` | Caught only by its corpus case's timeout: about a minute where its change's `check:fast` runs it, and in the nightly | A compile timeout of its own for a corpus case that says it, a few seconds |
| Two tests that compile a port inside bun's 5 s, lowering-labels' "React's addObjectDiffToProperties has no block MIR made" and js-builtins' "unwrap of a throwing call each branch makes" | Time out in `check:fast` when other worktrees load the Mac (load 35–48), failing a run that passes with each alone (8 s for the second's file) | A timeout that fits their compile, or the port compiled once per run and shared |
| The apps whose binding crates aren't the seed's (#1005): the pilot's, from the crates' npm packages, checked twice, in its `target/` and its `node_modules/.cache/rust-js/target`, under a key of their own; cargo-react's "inside" app, from copies; and its "versions" app, which counts what React's build script wrote | Each checks `webapi`, `react` and `js` again, 1.8 s of `webapi` alone, 3.4–4.1 s under load (#885); the pilot's 46 CPU-s of `check:fast --all`, its wall 52 s. The rest of the apps' targets are made from the run's seed (`seeded`, test/support.ts), and libraries, crates and cargo-workspace, the longest files now, use no binding crate | The pilot's second target made from its first, the same key; the longest files' own crates |
| A mutant's tests that don't end on CI's machines, `mir-tidy-named-temporaries-off` (nightly of 2026-10-27, its run stopped at 20 min) and `spill-hyphenated-name` (2026-10-30, 2 min 28 s over 7 JSX files, no test failed) | A nightly red with an inconclusive mutation; each is caught here in seconds | Their log's end is now printed with the verdict: read it the next time, and fix what it waited on |
| `test/node-bindings.test.ts`'s "node's modules are imported, and its globals called" | Builds the node crate in its own body, `buildNode()`, inside bun's 5 s: 7.8 s cold alone on the M3 Max (2026-11-02), 1.1 s once it's built; flaky in `check:fast` runs that start with no node crate built | `buildNode` in the file's `beforeAll`, as `buildReact` is, with a timeout that fits it |
| `test/architecture.test.ts`'s "each runtime helper is asked for, or needed by another" | 0.8 s alone; past bun's 5 s beside `check:fast`'s other jobs, failing runs that passed with it alone, until its timeout was raised to 30 s; it reads every source and each helper's JS on each run | The sources read once for the file, shared by its tests, not again per test |
| `test/runtime-package.test.ts`'s "room asked of a length, a resize's, a collect's or a hash table's, ends where wasm32's std ends it" | 2.5 s alone on the M3 Max (2026-11-02); killed at bun's 5 s in a loaded `check:fast --all`, its Node run reserving 117 MB | A timeout that fits it, as the snapshot tests declare |
| `test/support.test.ts`'s "a store keeps nothing below the disk's floor" | About 6 s on the Mac (2026-11-09): it prunes the machine's real stores, some 18,000 native builds in 10 GB, reading each one's size; past Bun's 5 s it timed out beside another file, so its timeout is `underLoad(60_000)` (#1108) | Prune a fixture store of its own, or keep each store's size beside it so a prune reads only what it removes |
| `test/runtime-package.test.ts` and `test/runtime-transpiled.test.ts` in one `bun test`, without `--parallel`, by hand | One test fails, "@rust-js/std is tree-shakable, a helper bundled alone" or an unhandled error between tests, where each file alone passes (15 of 15, 3 of 3, 2026-11-05); `check:fast` and CI run them under `--parallel`, one worker each, so only a hand-run combination meets it | Find what they share, their bundles' output directory or Vite's cache, and give each its own |
| The iterator matrix in full on the Mac, `ITERATOR_MATRIX=all` (ADR 1090) | 2,970 s in 4 parts at a load of 15–60 (2026-11-04), about 3.3 CPU-hours: 1,700 programs of 60 cases, each built natively and twice by the debug compiler, its compile the most of it | The release compiler for its builds, or a compile of a batch's two builds in one rust-js process; the nightly runs it on six machines |
| Whether a matrix row fails only in its batch, not alone (#826, #835) | A case's program alone (`program <n>`) can run as native where its batch, 60 cases in one program, doesn't, as one case's back step makes another's type knowing its back (`back_kept`'s per-type set, #814). Telling which takes the batch's program made by hand from `batches()` and `program()`, then finding the case that triggers it by compiling it beside each other case (60 compiles, run in parallel) | A `program <n> --batch` in `test/iterator-matrix.ts` that writes case n's whole batch, and a `--trigger` that finds the case that changes its form |
| Which Prettier and plugins lay out an app's JS, of each build's `rustc -vV` line (ADR 1003, #692) | 0.4 s of each build of React's port (17 packages) on the M3 Max, 2026-11-04, and of each editor check: `resolveConfig` of every module's `.js` and `.jsx` | Each config file's plugins read once, its `overrides` with them, not asked again per module |
| React's port regenerated, `bun scripts/react-in-rust.ts regen` | 26 s on the 12-core M6 after a change to `src/`, the compiler at `opt-level = 1` (75 s unoptimized, #896; 504 s before ADR 1100, #884): its crates a chain, each one `rust-js` on one core, `react-reconciler` 8 s, `react-dom-bindings` 5.7 s (7.8 s before #895), `react-server` 4.7 s, one after another | Profile `react-reconciler`'s and `react-dom-bindings`' compiles again (macOS's `sample`), now that tidy's `else if` chains are linear |
| `test/lowering-functions.test.ts`'s "a long function is lowered block by block" | 3 s alone on the M3 Max, 54 billion instructions for 3,000 `v.push(..)`s (24 billion for 1,500), the same at b80aedb1e and 707d7eea0 (2026-11-04); on CI 9 s, then 17 s, then past its 30 s timeout beside the test job's other files, so its timeout is 60 s | A lowering of a long straight-line body that costs less per statement: profile where its 18 million instructions a statement go |

## Working on `main`

- **Each change is a commit on `main`, pushed directly**: no branches, no
  pull requests. Its message is titled `feat(rust-js): …`.
- **A push that undoes a commit already on `main` is refused** by the
  pre-push hook `bun install` sets git to run (`.githooks/pre-push`,
  `scripts/push-guard.ts`): a commit made from an older tree, as `git
  reset --soft origin/main` after a fetch makes one, carries each file the
  newer commits changed back as it was: a file as it was before one of
  them, or a recent commit's whole change, in each file it touched, that
  applies to what's pushed again (f7d5e10ec's undoing of 02df83a40, files
  it edited too). Squash onto the merge base and rebase; to revert a
  commit on purpose, or take away all a recent commit added, a slow spot's
  row once it's fixed say, name it in the message ("This reverts commit
  <sha>").
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
- **A timeout that fits each local command** ([AGENTS.md](AGENTS.md#making-and-checking-changes)):
  about twice its usual time, so a hang ends; one past it is a hang or a
  slow spot to fix, never retried with more time.
- **A new worktree runs `bun install --frozen-lockfile` first.** Without a
  `node_modules` of its own, a worktree under `.claude/worktrees/` finds the
  checkout's above it, so its tests' JS imports that checkout's
  `@rust-js/std`, of another commit; `buildCompiler`
  (`test/support.ts`) fails a run that would.
- **An app a test builds with Cargo is kept between runs through
  `KeptApp`** (`test/support.ts`, #882): its files written through it, each
  only where it differs, and `done()` once they're written, so a run that
  changes none of what it's built from finds it built, as Cargo judges it.
  Never a fixture of its own kept some other way. A test that counts what
  its first build compiles, or that its target starts empty, keeps a
  fixture.
- **Generated JS is run by hand one way**: `bun scripts/run-case.ts
  <case.rs>` compiles it as the corpus's cases are and runs it natively
  and as JS, saying whether they agree (`--std-iterators`, `--tidy-until
  <pass>`, `--prepare-without <rewrite>` as the tests give them);
  `bun scripts/run-case.ts <case.js | dir>` runs JS already compiled. Each
  run is stopped, with what it started, past its time. Never a bare `node`
  or `bun` of generated JS: one that never ends outlives the shell, and one
  held a core for 28 hours, parented by launchd (2026-11-09).
  `check:fast` and `bun run task start` name what the loop left running so
  (`bun scripts/strays.ts`), and a machine with any isn't quiet.
- **A failing program is reduced, not by hand** (ADR 1107):
  `bun scripts/reduce.ts <case.rs> --build plain|std` shrinks it to a
  program that fails the same way, in parallel, in seconds to a minute; the
  metamorphic net's and the neighbour shuffle's failures come with theirs
  (`NET_REDUCE=0` to skip it). Start a P0 from the reduced program. One
  with `unsafe` code is reduced only with Miri installed (`rustup toolchain
  install nightly --profile minimal --component miri,rust-src && cargo
  +nightly miri setup`), which rejects each step that's undefined
  behaviour (#1104).
- **The inner loop is never delegated.** The failing test is seen to fail
  here, before the fix, and its mutations caught here, before the push.
- **Read every bless patch before committing it.**
- **A compiler change needs no cache cleared by hand.** Cargo is told the
  compiler's build and the `@rust-js/typescript` that prints the `.d.ts`,
  each by what it is (ADR 1003, amended for #675), and the app's Prettier
  and its config's plugins by their packages' manifests (#684, #692): a
  `yarn rust` or a regen after rust-js, Prettier or a plugin changes, at
  the same path or another, builds every crate again. A cache copied from another checkout is rebuilt too, as its
  crates' paths differ; copy none, as it's tens of GB ([slow spots](#slow-spots-still-to-fix)).
- **A red run is the change's.** Fix it at once, on `main`, and say what
  broke, with the failing job's log.
- **A runtime mutation runs beside `check:fast` in one checkout.** A
  mutation of a runtime helper (`src/runtime/*.js`, its entries in
  `scripts/mutations/runtime/`) runs its tests with its mutant's runtime
  installed in their own target (`ownRuntime` in `scripts/mutations.ts`,
  `test/packages-of.ts`), and the checkout's `std/runtime.js` is never
  rewritten. It was rewritten until #720, and a `check:fast` beside one
  ran its corpus parts and other test files against the mutant's runtime.
  A test that reads the package by path reads `stdDir` and
  `helpersModule` (`test/support.ts`), so it reads the run's own, never
  `join(root, "std")`.
- **`std/` is generated, and committed.** `bun run std` builds the
  compiler, then writes the helpers' `std/runtime.js`, `std/iter.js`, its
  `std/iter.js.map`, `std/manifest.json` and the entry, `std/index.js`. A
  change to a runtime helper, or to the lowering std's compiled bodies
  depend on, runs it, rebuilds (`bun run build`, which builds std's
  manifest in), and commits what it wrote
  with it: `test/runtime-package.test.ts` and `test/std-package.test.ts`
  hold them to what the compiler writes. Left stale, every iterator
  matrix run fails with "does not provide an export".

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
| `plan` | of a push, the commit its mutations are since, `main`'s last check whose mutations passed, or a check started by hand's `since`; their machines; and whether the change is the iterators' code (`scripts/iteratorSlice.ts`'s `iteratorPaths`) | 1 min |
| `lint` | typecheck, fmt, clippy, `cargo test` | 10 min |
| `test` | `bun run test`, or a shard of it, balanced by the last run's timings, `timings-*`, the corpus in parts beside it; a bless patch on failure | 9–14 min in 2 shards (run 40881358992), where the corpus in one file took 13–24 |
| `examples` | the Vite example's build | 10 min |
| `wasm` | build `rust-js.wasm`, then the snapshot and playground tests with it | 30 min |
| `rustc` | rustc's suite, a release build, then 6 shards and a report; a lists patch on failure | 20 min |
| `iterators`, `iterator-conformance`, `iterator-forms` | of a change to the iterators' code (`plan`'s `iterators`), each on a machine of its own, after a release build, the user's choice of 2026-11-05: the iterator matrix's slice (ADR 1090, #731), 64 fixed batches, of every source, adapter, consumer and panic site, of closures capturing what drops, and of a type stepped from its back beside its forward consumers; the conformance table's slice, 43 batches (ADR 1092); and every corpus case built with std's iterator structs (`ITERATOR_FORMS=corpus`, ADR 0908, #766); each in both builds against native Rust where it has two, 4 parts on one machine; skipped otherwise, and by the nightly, whose `matrix`, `conformance` and `forms` run all of each | on CI, the build cached: 120 s (matrix), 68 s (conformance), 298 s (forms), 171, 109 and 335 s each job (run 41977111573); a change to `src/` adds its 116 s build to each |
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

Its generated programs run each under Miri first (`FUZZ_MIRI=1`, ADR 0092,
amended), the nightly toolchain's, which its job installs. To run them so
on a Mac, once: `rustup component add miri rust-src --toolchain nightly`
and `cargo +nightly miri setup`; then `FUZZ_MIRI=1 FUZZ_START=.. FUZZ_SEEDS=..
bun test test/fuzz.test.ts`. Miri adds about 0.6 s a program; 720 seeds in
12 parts took 65 s (load 12 on the M6, 2026-11-09).

### `nightly.yml`, each night at 02:00 UTC+7, of `main`

Every test there is: `check.yml`'s jobs, its suite in 2 shards with the
files only the nightly runs (`--nightly`, ADR 1030), and every
mutation on 16 machines where a branch's check runs only its own, 600
generated programs, new ones each night, from the day's number, and every
corpus case after each of tidy's stages and without each of preparation's
rewrites on 6 machines, and the 600 programs so on 6 more (ADRs 0659,
1087), and every case of the iterator matrix on 6 machines of 4 parts
(`matrix`, ADR 1090, 2,970 s on the Mac in 4 parts at a load of 15–60,
2026-11-04). It's a
group of its own, so it neither cancels nor waits for a push's check.
`bun run ci:status` shows its latest run first; GitHub mails a failure to
whoever last changed the schedule. Of a public repository, its machines
cost nothing. Started by hand too: `gh workflow run nightly.yml`.

It casts the crates' and the port's nets too (ADR 1067): `crates`,
`bun scripts/crate-corpus.ts --check`, which fails where a crate does worse
than its row in `docs/crate-corpus.md`, the corpus's one list, each crate at
the exact release its row names, so a new release is measured by editing
its row, never by crates.io's publishing it. Every crate of each graph is
held to that crate's lock, `scripts/crate-corpus/<name>.lock` (`--locked`),
and `bun scripts/crate-corpus.ts --lock <name> ..` resolves a graph again
on purpose and measures it (ADR 1067, amended). A row marked with an open
issue, `runs (refused: #467)`, holds while the crate does just that. A
crate rust-js crashed on is printed as crashed. Until #511, regex-syntax
overflowed rust-js's stack, inlining a recursive type's `==` without end:
6.9 GB at 8 s in the debug compiler, and enough in the release one to shut
down a 16 GB runner even with Cargo building one crate at a time. Since
the fix, the largest process of the whole corpus is regex-syntax's, at
436 MB (debug) or 427 MB (release). Then `crater`, `bun scripts/crater.ts
--check`, each crate's own `#[test]`s run natively and as JS, a refused one
cut from the JS's copy and the rest run, failing on a wrong answer
`scripts/crater-decided.txt` doesn't list (ADR 1109); its largest process
is chrono's dev-dependency `windows_bindgen`, 9 GB, until #1096. Then `port`,
react-in-rust at `ports/react-in-rust.pin`, each crate compiled to JS again
and compared with what that commit committed, its summary listing each
file that differs and its first lines, the whole diff a `react-in-rust-diff`
artifact. An intended change to React's JS is accepted by moving the pin to
the port's commit that regenerated it. A red night of `main` opens an
issue, `Nightly red: <date>`, or comments on that day's (`issue` job), so
the issue watch gives it an owner.

### `readability-passes.yml`, the readability oracles, of a push that changes tidy or preparation

`test/tidy-stages.test.ts`'s nine cases, `test/getter-reads.test.ts`'s read
cases and the first six generated programs (`TIDY_STAGES=programs`), each
after every one of tidy's stages and without each of preparation's
rewrites that changed it (ADR 1087), of a push whose `paths` are
`src/prepare*`, `src/lower/mir/tidy*` or the oracles' own files. A sample:
the nightly runs every corpus case and the night's 600 programs so. On the
Mac, the nine cases took 90 s with their runs kept, six programs 150 s and
the read cases 16 s (2026-11-04, a debug build, the load near 13). Started
by hand too: `gh workflow run readability-passes.yml`.

### Of a change to the iterators' code: check.yml's `iterators`, `iterator-conformance` and `iterator-forms`

Three jobs at once, each on a machine of its own beside the suite, where
the check's `plan` says what changed since the last good check is the
iterators' code (`iteratorPaths` in `scripts/iteratorSlice.ts`, the one
list, which `test/architecture-iterators.test.ts` holds to the
architecture's owners of iterators); and in `check:fast` where the change
or the branch's since `main` is. The user's choice of 2026-11-05: all three
per push, each in a job of its own.

- **The matrix's slice**, 64 of its 1,787 batches (ADR 1090, amended for
  #731), fixed by the cases alone (`slice()` in `test/iterator-matrix.ts`):
  the fewest that hold every pairing of a source, an adapter, a consumer
  and a panic site, of closures that capture a `D` and of a type stepped
  from its back by `rfold`, `nth_back` or `rfind` beside its forward
  consumers. 89 s on CI in 4 parts.
- **The conformance table's slice** the same way, 43 of its 360 batches
  (`ITERATOR_TABLE=conformance`, ADR 1092), whose known cases are rows of
  the same list, `conformance:plain` and `conformance:std`. 62 s on CI.
- **Every corpus case built with std's iterator structs alone**
  (`ITERATOR_FORMS=corpus`, ADR 0908), but those `differs` lists: #766, a
  regression only that build showed, reached `main` where only the
  nightly's `forms` ran it. 298 s on CI in 4 parts, 183 s on the Mac at a load of 20–30.

Each job is about 5 minutes wall, its release build 116 s of it (run
41969266534). A case that differs from native Rust, in either build, and
isn't listed fails it; one listed that passes fails it until `bun
test/iterator-matrix.ts take-off` takes it off. `ITERATOR_MATRIX=#<issue>`
runs the batches that hold one issue's known cases, for its fixer.
`test/ci.test.ts` holds each setting a test file reads (`parts` in
`scripts/shard.ts`) to where it runs, per push or at night, so a part like
`ITERATOR_FORMS=corpus` can't be left to the nightly unsaid again.

### `react-in-rust.yml`, the React port, of a push that changes a binding

`bun scripts/react-in-rust.ts check`: react-in-rust at its pin, `cargo
check`ed natively against the pushed binding crates, `builtins/`,
`webapi/`, `node/` and `react/`, the `paths` its `on.push` names, so a
compiler push doesn't start it. No compiler is built. A binding's change
that breaks the port's Rust is red at its push, not at the porter's next
regeneration; it's fixed in the binding, or the port follows and the pin
moves. Started by hand too: `gh workflow run react-in-rust.yml`.

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
