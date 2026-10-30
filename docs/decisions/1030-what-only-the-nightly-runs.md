# 1030. What only the nightly runs

Status: Accepted.

Case ([0262](0262-when-rust-and-js-disagree.md)): none; this decides when
tests run, not what the JS does.

## Context

Each push to `main` ran the whole suite on two machines, 13 minutes each
(check run 40882867251). The corpus is the longest part of each machine's
run, about 9.8 minutes, and everything else, 3,116 s of CPU, runs beside
it on the same cores. 1,660 s of that is in tests no mutation names
(ADR 0093). The biggest of them, file by file:

| Files | CPU a push | Mutations naming them |
|---|---|---|
| `tidy-stages`, `iterator-forms`, `fuzz` | ~340 s | 25, run by the mutations themselves |
| `packages`, `create`, `npm-crates`, `release`, `stable`, `crates-io`, `registry-crates` | ~310 s | 8 |
| `react-versions`, the `*-coverage` ratchets, `editor-check`, `harness`, `history`, `limitations` | ~370 s | 7 |

The first group's whole runs are the nightly's already: every corpus case
after each of tidy's stages (ADR 0659), every case with std's iterator
structs (ADR 0908), 600 generated programs (ADR 0092). The second group
checks what a release packs, which qualification runs again (ADR 0094).
The third group checks ratchets and the test tooling, which change rarely.
None of these files failed in the 80 red checks before this decision. Over
the same runs, `crates`, `emission`, `next`, `pilot` and the lowering and
JSX files are where failures were found.

## Decision

**`nightly` in `scripts/shard.ts` lists the test files only the nightly's
suite runs.** `bun run test` leaves them out, and `bun run test --nightly`,
which `check.yml` runs when the nightly calls it, puts them back in.

- A mutation that names one of them still runs it, so each push's changed
  mutations still check what these files check.
- Qualification runs every test file, these included.
- A change to what one of them checks runs it locally, by itself:
  `bun test test/<file>.test.ts`.
- `test/ci.test.ts` checks that each listed file exists, and that the
  check passes `--nightly` to both of its suite runs, the bless's too.

## Consequences

- Each push's suite has about a third less CPU outside the corpus, and its
  corpus parts have that much less work beside them.
- A regression only these files catch is found by the next nightly, up to
  a day later, rather than by the push that caused it. Its fix is then a
  commit of its own.
- A file belongs on the list only if its whole run is the nightly's
  already, or it guards few mutations and has not failed a check. A file
  that starts catching pushes' regressions comes off it.
