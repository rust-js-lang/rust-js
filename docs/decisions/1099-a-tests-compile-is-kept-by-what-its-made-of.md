# 1099. A test's compile is kept by what it's made of

Status: Accepted. Extends [0101](0101-cargo-workspace-wrapper.md)'s
record of a build to a file compiled alone, and amends
[0567](0567-a-corpus-cases-runs-are-kept-by-what-they-ran.md), whose
compiles always ran. Keys by [1003](1003-a-compilers-build-is-its-identity.md)'s
identity; sound by [1098](1098-the-compilers-maps-are-walked-in-the-programs-order.md)'s
determinism.

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
