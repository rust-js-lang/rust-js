# 1099. A test's compile is kept by what it's made of

Status: Accepted. Extends [0101](0101-cargo-workspace-wrapper.md)'s
record of a build to a file compiled alone, and amends
[0567](0567-a-corpus-cases-runs-are-kept-by-what-they-ran.md), whose
compiles always ran. Keys by [1003](1003-a-compilers-build-is-its-identity.md)'s
identity; sound by [1098](1098-the-compilers-maps-are-walked-in-the-programs-order.md)'s
determinism. Amended (2026-11-07, #914): every store the loop keeps on
disk bounded the same way, and a fixture removed as its tests end.

Case ([0262](0262-when-rust-and-js-disagree.md)): none; how a build is
recorded and how the tests run, not what the JS means. The JS is the same,
byte for byte, with the record and without it.

## Context

`check:fast --all` keeps the 12-core M6's cores 93–94% busy (#881), so
it's bound by the CPU work it does: about 2,000 CPU-seconds with no change,
1,780 of them rust-js's own processes, 9,050 calls (measured with a shim in
place of the compiler, #874). 780 seconds are Cargo's builds of crates in
the cargo tests' own targets; 380 the corpus's compiles, 350 the other
files' compiles in their fixtures, 260 the rest. ADR 0567 kept the corpus's
runs, but its compiles ran each time: they were what's checked of the
compiler. Yet a run where the compiler's build is the same, a change to a
test or a doc, `check:fast` again after a rebase of others' test changes,
compiles everything again to the same JS.

Two things kept a compile from being kept. What it read wasn't known: a
case's `include_str!` of a file beside it, the crates of a `-L` directory, a
crate's `Cargo.toml` settings, a library's manifest; and a file compiled
alone refused `--emit=dep-info`, which only Cargo's build wrote. And the
same test compiled at another path each run: a fixture was
`target/fixtures/<pid>-x<random>/<name>_<random>`, and the path is part of
the JS, in its header and its map.

## Decision

**A file compiled alone writes the record of its build that
`--emit=dep-info=<path>` asks for,** as `make` reads one: its targets every
file it wrote, the JS, the maps, a library's metadata, the manifest,
`--std-coverage`'s report and `--prepare-fired`'s list, the record itself
aside; its prerequisites every file it read, rustc's record of the sources,
with each crate it loaded (`-Z binary-dep-depinfo`, set where it's asked),
the compiler, and what rust-js read itself: its libraries' manifests and
artifacts, the `Cargo.toml` its settings are read from, the manifest of the
build before, which says what's stale, and the JS files its bindings
import; and rustc's `# env-dep:` line for each variable an `env!` read.
rustc's driver writes it after the callbacks go on; rust-js's stop the
session, so they write it then (`rustc_interface::passes::write_dep_info`).
What it can't say all of is refused: a crate whose hooks, programs of its
own, run, and `--declarations`, whose TypeScript Node finds.

**Each compile a test makes through `test/child.ts` is kept by what it's
made of and given again where all of it is the same** (`test/compiles.ts`):

- **the key**: the compiler's build's identity (ADR 1003's, from
  `--version-json`), the arguments, where it runs, its environment but the
  harness's own settings (`unread`, which the compiler's sources are held
  never to read), and the names of the crates in each `-L` directory;
- **an entry** of the key for each set of files read, each by its content:
  given where each is as it was, its files written where they differ, and
  what it printed. A manifest of the build before that the entry didn't
  write makes it another compile;
- **never kept**: a compile that failed, one that read a file it wrote, one
  rust-js refused to record, Cargo's builds, a mutant's, a script standing in
  for the compiler, and a compile given `RUST_JS_COMPILE_CACHE_MB=0`, as the
  determinism net's are.

**A fixture is at the same path in each run**:
`target/fixtures/x<hash of its test file>/<name>_<n>`, the `n`th of that
name its process asked for, claimed by a link beside it naming its pid; one
a live process holds is passed for the next `n`.

**It's bounded**: 2 GB, `RUST_JS_COMPILE_CACHE_MB`; each use marks its entry;
past the cap the least recently used go, as a process's writes would pass it
and as `check:fast` and `bun run test` start, which also drop another build's
entries, never found again; below the disk's 40 GB floor nothing's written,
and what's kept is pruned until the floor's back or it's empty.

## Why

- **The output is a function of the inputs** (#866, ADR 1098): the same
  build, arguments, environment and files give the same JS. What a compile
  is made of is then all it can depend on, and the compiler, which read it,
  is the one to say what it read, not the tests guessing.
- **The record is rustc's and Make's own form**, `--emit=dep-info`,
  extended from Cargo's build to a file's, not another flag: a build tool
  or an editor watching a file's compile reads it too.
- **By content, not by date**: a fixture is written anew each run.
- **Measured** (12-core M6, `check:fast --all --profile`): 2 min 56 s with
  no change before; with the cache warm 1 min 47 s, and 1 min 50 s after a
  test's one-line change, the corpus's 12 parts 2–5 s each, not 33. After a
  one-line change to `src/`, nothing's found: 2 min 59 s to 3 min 23 s,
  within a run's noise of before plus the build. What's left is mostly
  Cargo's builds of crates (#874's slow spot).

## Alternatives

- **The path made the same in the key, swapped in the output.** A program
  can depend on its path's length or bytes (`file!().len()`), so a swap is
  a guess; the path is an input.
- **The tests list what a compile reads**: `include_str!`'s files, a
  crate's settings. A second catalog of the compiler's reads, wrong where it
  reads one more.
- **Keyed by the compiler's file**, its inode and date. `cargo test` links
  it again with nothing changed, and every entry was another build's.
- **Cargo's builds kept too**: their outputs are Cargo's whole target; the
  cargo tests are what checks that it rebuilds as it should.
- **A compile that fails kept too**, its diagnostics and its end (#886):
  measured first, with the cache warm and no change, the 207 a
  `check:fast --all` could keep took 12 s summed, about 1% of its CPU and
  under a second of its time, so not built: its record would need
  rustc's written as the crate's expanded and rust-js's own reads on every
  failing path (docs/dev-loop.md, dropped).

## Consequences

- A run after a change to `src/`, most of an agent's on the compiler,
  saves nothing; one after a change to the tests, the docs, a port or the
  scripts saves about 40% of `check:fast --all`.
- ADR 0567's "the compiles always run" holds of CI's runs, which have no
  cache, and of any run after a change to the compiler; elsewhere a compile
  is checked once a build.
- A compile with `--emit=dep-info=<path>` writes one more file, and creates
  and removes the staging directory beside its JS, as a library's metadata
  does.

## What holds it

- `test/compiles.test.ts`: a changed file read, option, variable or `-L`
  directory, and another compiler, compile again; a failed compile and one
  that read what it wrote aren't kept; the record's targets and
  prerequisites, the JS the same without it, and its refusals; the cap,
  oldest first, another build's dropped and the disk's floor; `unread`
  against the compiler's sources.
- `test/determinism.test.ts` compiles with the cache off.
- Mutations `file-record-without-crates`, `file-record-unwritten`,
  `file-record-without-settings`, `file-record-without-previous-manifest`,
  `file-record-of-hooks` (`main.rs`), `file-record-without-manifest-written`
  and `file-record-without-env` (`output.rs`).

## Amended: every store the loop keeps on disk is bounded (#914)

The cache's bound was this ADR's alone, and the loop's other stores had
none, or only an age. On the 12-core M6 one `check:fast --all` from an
empty `target/` left 12.3 GB: 7.3 GB of fixtures, as Bun's test runner
never emits `exit`, where each process was to remove its own; 2.6 GB of
the guide's apps, made anew each run and never read again; and React's
port, `scripts/react-in-rust.ts`, kept a directory in `$TMPDIR` for each
checkout, its worktree removed or not, growing 530 MB with each build of
the compiler. Seven orphans took the M6 under its floor twice in a day.

**Each store the loop keeps is bounded the same way** (DEVELOPMENT.md's
table): a cap, the least recently used going first past it, what's stale
pruned, nothing kept below the disk's 40 GB floor, and a test.
`test/bounded.ts` is the one floor and the one eviction; each store says
what its things are, how big and when last used:

- **this cache**: as above, 2 GB;
- **native programs' builds and the corpus's verified runs**
  (`test/kept.ts`): 3 GB and 256 MB besides a week unused, pruned as each
  `check:fast` starts, in a process of its own while the compiler builds,
  not once a day;
- **a test's fixtures**: removed as its file's tests end, by
  `test/setup.ts`'s `afterAll`, and as a script exits; one a killed process
  left, by the next process's first claim, taken as a claim is so one
  another process takes at once stays its own;
- **the guide's examples** (`bun run docs`): each removed once its output's
  read;
- **React's port's directories**: 4 GB together; one whose checkout is gone
  removed as any run starts; of those no run used in the last hour, the
  least recently used past the cap or below the floor, never the run's own.
  Its build is of one build of the compiler, as Cargo keys every crate by
  it and another's is never found again, and without rustc's incremental
  cache, 420 of a build's 540 MB, which no regen reads: after a change to
  the compiler each crate's compiled whole again.

Case ([0262](0262-when-rust-and-js-disagree.md)): none; what the loop keeps
on disk, not what any JS is. React's port regenerates byte for byte as its
pin committed it, with the incremental cache off.

Measured: the same `check:fast --all` used 5.1 GB at its most, not 12.3, and
left 1.6 GB, in 200 s, not 209; the regen leaves 215 MB after each change to
the compiler, not 640 MB and 530 MB more each, in 24 s, not 26.

Alternatives: the fixtures by age, which a tarball's dates defeat (above);
a removed worktree's port directory removed by a hook on `git worktree
remove`, which only a run that starts after it would see anyway, and which
no hook runs where an agent's session removes it; an app's
`node_modules/.cache/rust-js` bounded by `rust-js build` itself, a product
decision an app's watch loop may want the incremental cache for (left as a
slow spot, DEVELOPMENT.md).

Held by `test/support.test.ts` (the stores' caps and floor, a killed
process's fixtures, the port's directories) and `test/compiles.test.ts`.

## Amended: an app's cache is bounded by the same rule (#1057)

Case ([0262](0262-when-rust-and-js-disagree.md)): none; what's kept on
disk isn't what a program does. #914 left an app's cache,
`node_modules/.cache/rust-js`, unbounded, as a product decision: each build
of the compiler is another `rustc -vV` line, so Cargo builds every crate
again beside the last build's, and keeps both. The Vite example's had grown
to 17 GB of wasm builds, 13 of them rustc's incremental state, and took the
machine under its floor mid-run of the metamorphic net.

The user's decision (2026-11-09): cap it, least recently used first, with no
setting. `rust-js build`, `test`, `clippy` and the editor's check each run
Cargo through `inTarget` (tooling/build.js). A target of the app's cache
another build of the compiler made, or one from before its stamp, is
emptied first, but each `rust-js-in-source.json`, so what an earlier build
wrote beside the Rust is still known (`holdTarget`); then the cache's
targets are held to 2 GB, never the one in use, and to it alone below
5 GB free (`boundAppCache`). `CARGO_TARGET_DIR`'s target is the user's,
stamped but never emptied or evicted.

The eviction is one rule: `evict` moved from test/bounded.ts to
tooling/bounded.js, where the shipped tooling can use it, and takes its
store's floor, 40 GB of the loop's stores (test/bounded.ts's `floorBytes`)
and 5 GB of an app's. React's port's `ownBuildOnly`, which emptied its
build of another compiler's, is gone: `rust-js build` does that now for
every app. Held by `test/in-source.test.ts`' two tests of it; the tooling
has no mutations, as #914's stores have none.
