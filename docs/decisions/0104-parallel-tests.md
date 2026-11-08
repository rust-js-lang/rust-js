# 0104. The tests run side by side, each native program built once

Status: Accepted. Extends [0017](0017-differential-testing.md) and [0088](0088-corpus.md).

Case: N ([0262](0262-when-rust-and-js-disagree.md)).

## Context

`bun test` took 285 seconds, one file after another. Bun runs test files
side by side with `--parallel`, a worker for each core, and that took 153
seconds and failed 21 tests:

- **Bun's five seconds.** A test that takes three alone takes more beside
  thirteen others.
- **Playwright refused to run.** A worker says it's Jest's
  (`JEST_WORKER_ID`), and Playwright won't run inside Jest.
- **The compiler went missing.** Each file built it, `cargo build`, and
  Cargo links `target/debug/rust-js` again even when nothing's changed,
  so for a moment it isn't there for the other files.

And the workers mostly waited: the CPUs were a seventh busy. A native
program's binary is new each run, and macOS checks each new one as it
first runs, which takes half a second where building it takes a tenth, one
at a time for the whole machine. The corpus, serde's cases and the
generated programs built each of theirs every run.

## Decision

**`bun run test` runs the files side by side** (`bun test --parallel`),
the slowest first, by how long each took last time
(`target/test-timings.json`); `bun run test:serial` runs them one after
another.

- **A test may take a minute** side by side (`test/setup.ts`, loaded
  before each file), and one after another by `--timeout`
  (`test:serial`): Bun loads the setup once then, and sets each file's
  back to its five seconds. A bare `bun test` has those five.
- **The compiler is built once for the run, before Bun starts:** `bun run
  test` and `test:serial` build it, then run Bun with it given,
  `RUST_JS_COMPILER`, so every process of the run has it from the start,
  and nothing builds it again. Set later, by a test file, a child it
  spawns wouldn't have it: Bun gives a child the environment it started
  with. A file that built nothing ran the formatter, and a file its tests
  in a `bun test` of their own, which each built it, and Cargo took it
  away from the files beside them. Found in review.
- **A bare `bun test` builds it too** (`buildCompiler`, from
  `test/setup.ts`, before any file's tests): the first file builds it,
  and the others wait for it. The build is claimed (`once`), by a
  directory of the run's own, `target/tests-build/<run>`, the run being
  its workers' parent by pid and when it started: made whole, with its
  process's pid in it, and moved into place, which only one can do. It's never taken over: a build that
  failed, or whose process ended, is an error for the others, and so is
  waiting ten minutes, and a run's claim is no other run's.
- **A native program is built once, not once a run** (`nativeBinary`): its
  binary is kept in `target/native-cache/`, named by a hash of rustc's
  version, its flags, its source and where it is, since
  `include!("cases.rs")` is of the file beside it, and the libraries it
  links; and used again while none of what it was built from changes.
  That's also what rustc says the build read, its dep-info: each file an
  `include!`, a `mod` or an `include_str!` reached, however deep, whose
  hash is checked each time, and each variable an `env!` read. A new
  build is kept beside those before, which stay: another file may be
  running one.
- **A wrapper is where its text says** (`contentDirectory`), so the same
  program is the same kept binary in any run: a corpus case's includes
  the case where it is, so what it reads beside it is found as the JS's
  compile finds it, serde's cases are copied beside theirs, and a
  generated program is written where its text says. Two files
  building one program write one source, so it's replaced whole, a new
  file moved into place (`writeWhole`), never rewritten as the other's
  rustc reads it. A binary's
  first run may take a minute, not ten seconds: it waits its turn to be
  checked.
- **A file is what runs by itself,** so one taking most of the time is
  split by what it tests: `crates.test.ts`'s Cargo tests are
  `cargo-workspace.test.ts`, `cargo-in-source.test.ts` and
  `cargo-react.test.ts`, sharing `test/crates.ts`.
- **Playwright is run without `JEST_WORKER_ID`.**

## Why

- **The time is the slowest file's, not the sum.** 60 to 70 seconds, with
  the cache warm, and 195 to 210 one after another; a first run, building
  every native program, 142.
- **A binary is the same program each time.** A native program's output
  is the oracle's, and it's run each time; only building it and its first
  check are saved, and a change to anything it's built from builds it again.

## Alternatives

- **Copying a case beside its wrapper,** so a generated program, written
  to a new directory each run, is one binary: it cut what the case reads
  beside it off, which the JS's compile still found. Found in review.
- **A lock, taken over when its holder has ended:** moving a lock aside
  can't tell whether another has just taken it over, and two held it.
  Found in review. A run's own claim is never another run's to take.
- **Replacing a kept binary with a newer build:** another file, given it,
  found it gone. Found in review.
- **Keeping each native program's output, not its binary:** it would save
  running it too, which takes no time, at the risk of an answer the
  program no longer gives.
- **Only the tests a change affects (`bun test --changed`):** Bun follows
  what the tests import, and no test imports the compiler; they run it.
- **Turning macOS's check off for the terminal:** it's the user's security
  setting, and the cache makes it matter only once.

## Consequences

- The files still wait on each other more than they compute: 180 CPU
  seconds take 60 to 70. What's left is mostly Cargo's lock of its package
  cache, and the native programs still built outside `nativeBinary`, a
  few in `crates.test.ts`, `link.test.ts` and `compiler.test.ts`.
- `target/native-cache/`, `target/native-sources/` and
  `target/tests-build/` grow with each program changed and each run; it's in
  `target/`, and removing it only costs building them again.

## Amendment: the corpus runs in parts beside the rest

The corpus is one file, `test/corpus.test.ts`, of 1,300 cases one after
another, so Bun runs it as one worker, and `bun test --shard` gives it to
one shard whole. On CI it was most of its shard's time, 787 to 1,462
seconds in the day's runs (2026-10-29, their `timings-*`), while the
shard's other files were done in about four minutes and three of its four
cores sat idle; twice running it stopped at the suite's 25-minute
deadline (`scripts/test.ts`), main red with nothing wrong. The compiler
hadn't slowed: 123 of its cases compiled in 11 and 12 seconds, by that
morning's compiler and by main's.

**`bun run test` runs the corpus in parts, each a process of its own,
beside the rest of the suite** (`RUST_JS_CASES=i/n`, `test/support.ts`),
as `check:fast` has: one for each two cores, `caseParts`, one answer for
both, and of a shard of `m`, its `1/m` of `m` times as many,
`caseShardParts` (`scripts/shard.ts`), so each part runs in exactly one
shard and what isn't a case, in the first part, in the first shard. The
rest is sharded by Bun as before, the corpus left out of it. Held by
`test/ci.test.ts`'s "each shard runs its share of a file of cases'
parts, each part once".

## Amendment: a native program is the checkout's, whichever target its tests run in (#875)

#875 asked for native programs' outputs to be kept by their sources, so a
comparison runs native Rust once. Measured on the 12-core M6 with each
binary kept, running them is not where the time is: of the corpus's 1,813
native runs took 6.1 CPU-seconds of a 28-second run in 12 parts, about
336; the points table's 105 batches 3.5 of about 156; the unwind table's
58, 1.3 of 120; the matrix's and the conformance table's slices 0.4 and
0.2 of about 110 and 70. Kept outputs would save under 2% of any of them, so
they aren't kept.

What was paid was the builds, about 0.18 CPU-seconds each, where a binary
was kept by a path that changes: `nativeBinary`'s key holds where the
wrapper is, as an `include!` of a file beside it reads there, and
`contentDirectory` put it under the tests' target, which a mutant's tests
and each test the test map records have their own of (`RUST_JS_TARGET`).
So each mutant built again every native program its tests ran: 5,804 of
the 10,782 mutations name corpus cases, 12,600 of them. And each recording
of the test map built 3,642 programs into the checkout's store, which it
shares, by paths deleted as each test ended: 2 GB left in a store held to
3 GB, evicting the programs the next check runs, which it built again,
322 CPU-seconds of the corpus's.

**The wrapper and the kept binaries are the checkout's `target/`**
(`checkoutTarget`, `test/packages-of.ts`), whatever `RUST_JS_TARGET` says:
nothing in them is the compiler's. A mutant's corpus cases run 20 to 30%
faster (one case 0.43 s to 0.34 s; seven 2.07 s to 1.49 s; sixty 16.6 s to
11.6 s), and the test map shares them as it meant to. The wrappers are a
store of their own, held to 64 MB and a week unused like the rest
(`test/kept.ts`); they grew without a bound. Held by `test/support.test.ts`'s
"a native program is kept once for any target, and built again by another
toolchain".

## Amendment: the native programs are the machine's (#1047)

The checkout's store held each worktree's builds, to 3 GB. The metamorphic
net builds about 14,000 programs, 7 GB (ADR 1104): a worktree's next
check pruned most of them, and every task, each in a worktree of its own,
built them all again, as it built the corpus's.

**The kept builds are the machine's, `~/.cache/rust-js/native-cache/`**
(`nativeCache`, `test/kept.ts`), held to 12 GB, the least recently used
going first and none kept below the disk's floor. A build is the same in
any checkout of the same sources, as `--remap-path-prefix` leaves the
checkout's root out of it; so what keys it is said from the root,
`<root>/` (`portable`): the wrapper's source and its place
(`contentDirectory`), the flags, and each file the build read. A library
a flag names, `--extern metamorph_lib=<rlib>`, is keyed by what it holds,
not its time: the net builds its `.rlib` anew on each run, so each of
`library`'s programs was built again every time. Held by
`test/support.test.ts`' "a native program is kept by what it was built
from, said from the checkout's root". The wrappers stay the checkout's,
64 MB.
