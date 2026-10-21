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
