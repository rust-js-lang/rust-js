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

## Amendment: std by what the case's JS can run (#2352, the lead for Tu, 2026-11-23)

Case ([0262](0262-when-rust-and-js-disagree.md)): N, as above.

"A change to a runtime module … runs every case again" was most of what
made the corpus cold. Of main's 902 commits from 2026-11-16 to 11-23, 92
changed something every case's key held: 59 changed std/runtime.js, 17
changed only std's `iter`, `io` or `net` modules, and 16 changed the
harness. A cold part took 85 s against a warm part's 23 s on the MacBook
Pro (part 4/10, 290 cases). The extra time was all JS runs and the Vite
bundle. A new compiler binary voids nothing: its key isn't in a run's,
and a part after a trivial compiler change took 25 s.

So **a run's key holds the std code its JS can run, not all of std**
(`reached`, `test/verified-runs.ts`; `shared(entry)`). That is one bundle,
by Bun's bundler, of an entry that re-exports the JS and the std helpers
the harness names (the oracle's `$isPanic`, `$raisedAt`, `$unwindsOn`,
which a run calls besides the JS). Std's modules are served by their names
in std, `/@rust-js-std/…`, not as the files they are. The bundle drops
whitespace and comments and keeps every name. Where the JS can't be
bundled, the key holds all of std, as before.

**Why the bundle covers every std function the case can run.** A run
loads the JS, which imports `@rust-js/std`. Its `index.js` re-exports
every module, so loading it evaluates every module. The harness imports
`runtime.js`. Code runs only when evaluation calls it, or when code that
already runs reaches it.
- **Evaluation:** the entry imports std as the run does, and no
  `"sideEffects": false` applies, since the modules aren't bundled as
  files. A top-level statement or initializer the bundler can't prove pure
  is kept, in every module, even one the JS imports nothing from (test:
  `b.js`'s registered function).
- **Reaching:** code reaches other code by name: a call, a reference, an
  object holding methods. That includes a dictionary, a vtable,
  `$stdSites`, or a class's prototype. The bundler keeps each name kept
  code mentions, transitively, and keeps an object whole once it's
  mentioned: it doesn't drop properties. So dynamic dispatch, a property
  read off a kept object, reaches only kept code. The other ways code can
  be reached without a name are `eval`, `Function` and `import()`, and std
  has none (`test/verified-runs.test.ts` holds that).

The shipped run already depends on this: it runs Vite's tree-shaken
bundle, and that bundle is a subset of this one, which also keeps every
module's evaluation. Bun's version is in the key, so a bundler that shakes
otherwise is another key. And the nightly re-runs 2% of the verified runs
it's given, above.

- **Kept by what it's made of**, `std-reach/<key>/reach.json`, the
  machine's, held to 64 MB. The key is all of std's text, the harness's
  names and the JS's text. Each other file the bundle read, by Bun's
  metafile, is checked by its digest when the record is read again. A
  bundle costs about 12 ms a case, a warm part's compile about 76 ms, so
  it's made only when std or the JS changed. A warm part with the reach
  kept took 3 s, its compiles kept too. A mutant keeps its reach in its
  own target, as its runs are kept.
- **Std's modules are its own folder's `.js` files**, not what a
  `node_modules` of it links, which in a mutant's workspace links std
  itself.

Measured over those 92 commits, using 150 single-file corpus snapshots at
each commit against std at the commit and its parent: before, each
voided every case, so 92 whole-corpus re-runs; after, 37 (−60%).
- runtime.js commits: a median of 14% of the cases, mean 28%;
- `iter`/`io`/`net` commits: median 3%, mean 26%;
- harness commits: all of them, until the harness is keyed by the code
  that runs and compares (#2352's second part).

A change to `$stdSites`, one object, reaches every case that panics at a
std site, as does one on every program's print path.
