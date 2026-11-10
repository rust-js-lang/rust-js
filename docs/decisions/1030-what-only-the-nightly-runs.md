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

## Amendment: what a change can leave untrue unseen (2026-11-10)

Of the list's files, two kinds keep a record that a change makes untrue
without failing anything the change runs: `test/limitations.test.ts`,
each of the guide's refusals compiled to see it's still refused, which a
lowering that lifts one, or a new limitation whose example doesn't
compile, makes red; and the bindings' coverage ratchets, `next-`,
`react-`, `webapi-` and `node-coverage`, which a binding added or taken
out makes red until blessed. On the list, such a change was found by the
next nightly: 5096475be's limitation, whose example used a `groups` it
never declared, was red until then.

They're off the list. Each push's suite runs them, and plain
`check:fast` runs `limitations` where the compiler or std changes, about
7 s of CPU, and the ratchets on every change, about 15 s. The rule
above is narrowed: **a file whose record a change can leave untrue isn't
on the list**, however rarely it has failed. The 23 left are nets the
nightly runs in full (tidy's stages, the read cases, the iterator forms
and matrix, determinism, the `&mut` and `None` matrices, the object
model, the declarations' callers, fuzz's seeds, match trees), what
qualification runs again (the release's packaging, `create`, the npm and
registry crates, `stable`, `crates-io`, `compat-versions`), React's
releases, and the tooling's own checks (`editor-check`, `harness`,
`history`): a failure of the first is a wrong answer the net finds, which
the nightly's whole run is for, and the rest check what a lowering or a
binding doesn't change.
