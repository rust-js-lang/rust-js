# 0567. A corpus case's runs are kept by what they ran

Status: Accepted. Extends [0088](0088-corpus.md) and
[0104](0104-parallel-tests.md).

Case: N ([0262](0262-when-rust-and-js-disagree.md)): how the tests run,
not what the JS means.

## Context

Each corpus case is compiled twice, as a program and as a library, and
run four times: natively, under Node as compiled and as shipped, bundled
and minified by Vite, and under Node as a library, each run compared with
native Rust's (0088). The compiles are what's checked of the compiler.
The runs check the JS, and the JS of a case rarely changes: of the last
two hundred commits to the compiler, 86% changed two of 686 snapshots or
none. Yet every run of the corpus ran every case four times: Node started
three times a case, about 350 ms of a case's 900, and a third of
`check:fast`'s CPU time.

## Decision

**A case's runs are kept once verified, by what they ran, and the same
again isn't run again.** The key is of the case's source, the JS rust-js
made of it, every file, its header's path (the run's folder) made the
same, native Rust's output and how it ended, and what every case's runs
share: the runtime modules the JS imports, `@rust-js/runtime`'s and
`@rust-js/std`'s; the harness that compiles, runs and compares; and
Node's, Bun's and Vite's versions. The program's runs are one key, the
library's another. A key is recorded only when the runs it's of agreed
with native Rust, or, of a case rust-js gets wrong for now, still didn't.

- **The compiles always run**, and the snapshot is always checked: they
  are the test of the compiler. Native Rust always runs too: its binary
  is kept (0104), the run is 15 ms, and its output is part of the key.
- **Kept as native programs are**, `target/corpus-cache/<key>/passed`,
  each use marked, a week unused dropped (`test/kept.ts`), by `bun run
  test` as it starts.
- **A mutation's run keeps nothing shared**: its target is its own (0093),
  so no run of a mutant is skipped for one the compiler verified.
- **CI verifies everything**: a fresh machine has no cache. The cache is
  the inner loop's.

## Why

A run whose every input is the same gives the same answer. Keying by the
inputs, not by time or by the compiler's binary, is what makes a change
to the compiler cost only what it changed: a case whose JS is the same is
verified by its earlier run; one whose JS differs is run. A faster inner
loop ships faster.

## Alternatives

- **Skip when the JS is its snapshot.** A snapshot is blessed, not
  verified: a blessed snapshot whose runs failed would never run again.
- **Key by the compiler's binary.** Every build changes it, so nothing
  would ever be found.
- **Cache the native run too.** It's 15 ms, and its output in the key
  keeps the key honest about what the runs were compared with.

## Consequences

- `check:fast` after a change that leaves the JS of most cases as it was
  runs their compiles and skips their runs.
- A change to a runtime module, the harness, or a runtime's version runs
  every case again, as it should.
- `test/support.test.ts` holds the key and the record.

## Amendment: the compiles kept too (ADR 1099, #874)

Case ([0262](0262-when-rust-and-js-disagree.md)): N, as above.

"The compiles always run" no longer holds of every run: a case's compile
is kept by what it's made of, the compiler's build, its arguments, its
environment and every file it read, as its record of the build says, and
given again where all of it is the same (`test/compiles.ts`). They still
run on CI, which keeps nothing, after any change to the compiler, whose
build is then another, and in the determinism net, which keeps none. The
snapshot is still checked of the JS each run, compiled or given again.

## Amendment: the machine's, and the metamorphic net's runs too (#1047)

A run's key is what it ran, the JS and native Rust's output, not where:
so the verified runs are the machine's, `~/.cache/rust-js/verified-runs/`
(`verifiedRuns`, `test/kept.ts`), every worktree's, as each task starts in
a worktree of its own, held to 512 MB. A mutant's tests keep theirs in
their own target still, as above, and each test the test map records
reads the machine's.

And the metamorphic net's runs are kept so (ADR 1104, amended): a
rewrite's program, its JS, native Rust's output and end, `shared()`, and
the net's harness. After a change to the compiler, the net's whole run on
the 12-core M6 is 299 s, not 501 s.

## Amendment: native Rust's runs kept by what they were built from, and CI keeps them (ioCF, the lead for Tu, 2026-11-16)

Case ([0262](0262-when-rust-and-js-disagree.md)): N, as above.

"CI verifies everything: a fresh machine has no cache" was what made CI
slow. On the Mac mini, quiet, the corpus's part 1/8, 324 cases, took 236 s
with the native and verified-run stores cold, as each CI job starts; 44.8
s with only the natives' builds warm; 5.6 s with both. The natives were
80% of a cold part, and each test shard's corpus parts are its longest leg
(check 43740041639). Their binaries can't be kept on CI: the corpus's
alone are some 1.4 GB, and the repository's caches were at 10.3 GB of
their 10.

So "Cache the native run too", an alternative above, is taken, by its
build's key, not its output: **a native program's run is kept beside what
its build read** (`test/native-outcomes.ts`, `native-outcomes/<key>/<the
build's inputs>/run.json`, the machine's, held to 256 MB). The key is
`nativeBinary`'s, which already decides when a build is given again:
rustc's version, the flags, the wrapper's path and the source, each file
a flag names by its digest, and each file of a directory `-L
dependency=` names, which rustc's dep-info doesn't list (new here, so a
kept binary isn't given against another build of its crates either). And
what the build read is checked as a kept build's is: each file rustc's
dep-info says it read, by its digest, and each variable an `env!` or an
`option_env!` read (`# env-dep`). A chain of crates' program
(test/crates.ts) is kept the same way, by `nativeCrates`' key.

- **Only of a run that says the same each time.** Not a case's whose run
  reads the clock, a random seed, the file system, its arguments, its
  environment, its process's id or its input: each carries `//@
  nondeterministic: <why>` (test/corpus-cases.ts's directives, one reading
  of them), and `test/native-outcomes.test.ts` holds every corpus case
  whose code names such an API to it; a rewrite of one, the metamorphic
  net's, is judged by its case's (`of`). Not one stopped by its time or
  its memory. Not a run asked for afresh: the net's runs again, for
  another of the answers a hash table's seed gives (`fresh`).
- **CI restores them, and only `main` saves them**: each job that runs
  native programs restores the newest its own name kept, the test shards
  and the nightly's nets (`.github/actions/native-outcomes`), with the
  corpus's verified runs beside them; a branch's run restores and saves
  nothing. A few MB a job, 3 MB of outcomes for a corpus part of 324 cases.
- **The run's key holds the memory it's given**, `nativeCommand`'s share
  on macOS, its `ulimit` on Linux: a run that ends within 2 GB may be
  stopped within 200 MB. On macOS the key holds that it's the share, not
  its bytes (`nativeLimitKey`, #2340). The share is of what's free as a test
  starts, so its bytes moved with the load, and a run kept beside one busy
  check was never found beside the next. A run stopped past its share is
  never kept, so a kept one ended within what it was given. A reducer's variant is run afresh, its check held
  to a share of memory as a whole process (#1306), and a loud rewrite of a
  case marked `nondeterministic` isn't made: its events count what each
  run is given, the calls `env::vars().find(..)` makes before `PATH`, of an
  environment the native run's shell and the JS's runtime each order their
  own way.
- **The nightly checks them**: a random 2% of the kept runs it's given
  (`RUST_JS_NATIVE_RECHECK=0.02`) are built and run again, up to eight
  times, and one that never says what was kept fails its test, a key that
  misses an input. Eight, as a program whose output std leaves to a hash
  table's seed may say another of its answers first. And 2% of the
  verified runs it's given are run again and compared, not taken as
  verified.
- **The verified runs' key is as it was**, kept on CI beside the outcomes:
  it holds the JS the compiler wrote, so a change to the compiler that
  changes what a case runs misses; @rust-js/std's modules, its runtime's
  helpers among them; the harness, `test/native-outcomes.ts` added; and
  Node's, Bun's and Vite's versions. Not the compiler's binary, which every
  push changes (Alternatives, above).

Measured on the Mac mini, quiet, as CI would run with them restored: the
corpus's part 1/8, the native and verified-run stores cold and the
outcomes kept, 43.1 s, not 236 s; with the verified runs kept too, 3.5 s.
`test/crates.test.ts`, all cold 115.9 s, the outcomes kept 13.2 s.
