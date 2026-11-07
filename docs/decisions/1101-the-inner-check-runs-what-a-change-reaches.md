# 1101. The inner check runs what a change reaches

Status: Accepted (#877). Amends [1093](1093-the-check-before-a-push-is-the-pushs-check.md):
plain `check:fast` is the inner loop's check, `--all` still the check
before a push and unchanged. Records with
[0093](0093-mutations.md)'s kind of proof in reverse: what each test runs
of the compiler, not what each mutation breaks.

Case ([0262](0262-when-rust-and-js-disagree.md)): none; which tests the
inner loop runs, not what the JS means.

## Context

Plain `check:fast` runs after each edit. Of a change to `src/` it ran the
corpus in 12 parts and nine suites by the change's paths, each suite one
`bun test` of its files one after another: lowering's 74 files 70 s, the
last job of every run, the cores half idle 44% of it. On the 12-core M6 a
one-statement change to tidy took 79 s, to drops 85 s, to the bindings
77 s, to the iterators 153 s and to the runtime 136 s, these two with the
iterator rows they name (2026-11-07).

The suites ran whatever the change's paths said, the same for a change
to a function every compile runs as for one only a closure's capture
reaches.

## Decision

**Each plain suite's file is a job of the pool's**, as each file is in
`--all`: the suites (`scripts/shard.ts`'s `plainSuites`, the one list)
run side by side, said as their suite.

**A test map says which tests a change can affect.** `bun run test-map`
records it of HEAD (`scripts/testMap.ts`): a build of the compiler that
counts each function's runs (`-C instrument-coverage`), given to each file
of the plain suites and each corpus case, through a stand-in
(`scripts/testMapCompiler.ts`) that keeps the functions each compile ran,
a bit each, for the test it ran for: a corpus compile for the case its
wrapper includes. Kept in `target/test-map/maps/<commit>.json`.

Plain `check:fast` reads the newest map of HEAD or an ancestor and the
working tree's difference from its commit. Where each changed line is a
function's counted code, it runs the files and cases that ran one of
those functions, and those that run on every change; a changed plain test
file or corpus case runs itself, and a case the tests that read the
corpus's files. Anything else and it runs what the paths say, as before,
and says why.

## Why a skip is sound

A test runs the compiler's code; a function it never ran, changed, can't
change what it runs, so it ends as it did at the map's commit. That holds
only for a change to a function's code, and only where the test runs
nothing the map didn't count. So:

- **A changed line must be counted code of a function** (llvm-cov's code
  regions), joined over what's the function's own between them: a `use`,
  a closing brace, a comment, a pattern. A type, a constant, an attribute,
  a derive, a line between two functions, a `#[cfg(test)]` module: the map
  can't say. Lines added between two must both be one function's code, or
  of one nested in the other, a closure in it. A change that writes an
  item (`impl`, `const`, `static`, ...) inside a function can't be said
  either: an `impl` nested in a function is used outside it.
- **A file `include_str!` reads** is the line that reads it.
- **Only `src/`'s code.** A test's helper, std, the runtime's package,
  Cargo's files, the toolchain, a doc a test reads: the map can't say. So
  any of those changed since the map's commit, committed or not, and the
  check runs what the paths say until the map is recorded again.
- **Each test records alone**: its own `RUST_JS_TARGET` and std sysroot,
  and no compile cache, so a crate one test built isn't read by another
  unrecorded; the corpus's native builds and verified runs, which no
  compile of rust-js's makes, are shared. The native builds are the
  checkout's in any target (ADR 0104, amended, #875): kept by a path in
  the test's own target, each recording built 3,642 again, and left them.
- **Every run is counted, or the test runs always.** The compiler runs
  itself again for each crate of crates.io it tries: each process writes
  its own counts (`%p`). A run given no `LLVM_PROFILE_FILE` writes its
  counts where the build says (`__llvm_profile_filename`), and the
  recording fails. A compile stopped or aborted, which writes none, a
  function the table doesn't name, a test that failed, or one that ran no
  compiler, runs on every change. So does one that reads the compiler's
  sources as text (`readsSources`: the architecture's tests, the mutation
  lists).

## Measured

On the 12-core M6 (2026-11-07), five one-statement changes, plain
`check:fast` each, a statement added to tidy's `guarded_chains`, an
iterator's `collects_in_place`, drops' `glue_place`, the runtime's
`helper_of` and the bindings' `is_nullable`:

| Change | Before | Each file a job | With the map | What the map picks, of 101 files and 1,809 cases |
|---|---|---|---|---|
| tidy | 79 s | 57 s | 60 s | 101, 1,805 |
| iterators | 153 s | 144 s | 83 s | 7, 62 |
| drops | 85 s | 63 s | 59 s | 88, 1,231 |
| runtime | 136 s | 123 s | 121 s | 80, 1,538 |
| bindings | 77 s | 53 s | 56 s | 97, 1,631 |
| together | 530 s | 440 s | 379 s | |

The iterators' and the runtime's runs are the rows they name of the
iterator matrix too, which the map doesn't choose (#818). Each file a job,
the cores are 93–95% busy, so what's left is the CPU of what runs: a
function every compile runs, tidy's, picks all of it.

Of the catalog's 9,070 mutations whose own tests are the plain suites' or
the corpus's, the map can say for 8,981 and picks each one's own tests for
all of them: of an iterator's, a median 25 files and 236 cases; of drops',
44 and 500; of tidy's and the runtime's, nearly all. A deliberately broken
module of each area failed plain `check:fast` with the map: an iterator's
`extend` (20 files and 37 cases picked), a closure's drop (12, 65), tidy's
inlining (all), the runtime's `helper_of` (80, 1,538) and its `$at` (all),
a binding's keyed test (9, 4) and a WebIDL bound (14, 0). Of each but
tidy's, every plain file and every case was run too without the map:
each one that failed was one the map picked.

## Alternatives

- **By module, from `use` or the call graph**: every module of a compiler
  reaches nearly every other; nothing would be skipped.
- **Line coverage, not functions**: a test that took another arm of a
  `match` ran its patterns, which coverage counts as the arm's; a change to
  a pattern it never matched could make it match. A function it entered is
  the unit whose change can't reach a test that never entered it.
- **Recording in every plain check**, the map kept current: the counting
  build is slower, and each run's records would mix commits.
- **A map for a change to `docs/` or a test's helper**: what a test reads
  of them isn't counted; Bun's `node:fs` can't be watched from a preload for
  the ESM imports the tests use.

## Consequences

- `bun run test-map` after `git reset --hard origin/main`, and after
  committing a change outside `src/`: about 130 s on 12 cores.
- The map's build, `target/test-map/build`, is kept for the next
  recording, one build as `target/debug` is; the maps, three, each about
  7 MB, the least recently used going first.
- `--all` is unchanged: what a push's check runs.

## Amended: the corpus with std's iterator structs, by case (#876)

A change to the iterators' code runs the corpus built with std's iterator
structs (`ITERATOR_FORMS=corpus`, scripts/iteratorSlice.ts), every case,
29 s at 4.7 cores of plain `check:fast`'s 83 s for an iterator change. The
map records that build too, each case a test of its own
(`caseUnit`: `test/iterator-forms.test.ts test/corpus/<case>.rs`), as
its compiles take other paths than the plain build's, so the plain build's
picks can't stand for them. Plain `check:fast` runs the cases it picks
(`run forms <parts> <list>`), none where it picks none; a change to the
file runs each, and to a case, that case too. `--all` runs every case, as
before.

- The iterator change of the five: 83 s, now 73-74 s twice, the job 29 s,
  now 2 s (74 of 1,809 cases); recording 132 s, not 110 s.
- A closure's drop broken: 46 of the build's cases fail run over every
  case with no map, each of them among the 81 the map picks; an
  iterator's `extend` and an `Option`'s `then` broken fail none of them,
  of 54 and 397 picked.
- What's left of an iterator change is the rows it names (#818), 59 s of
  the 74, sized to their budget of time, and the two fixed slices, 16 and
  10 s. Of `--all`'s about 2,270 CPU-seconds with an iterator change, the
  rows are 385, the build with std's structs 160, the slices 128; run
  again with the same build, 124, 13 and 30, the compiles kept (ADR 1099)
  and the native programs too. The rows the two tables share: 2,235 of
  the conformance table's 21,594 cases are the matrix's of the same code,
  but in other batches, whose make-up changes a compile (#826, #835), and
  the table's budget is 40 s: at most 4 s, not taken.
