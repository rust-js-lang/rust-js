# The development loop's ledger

Every experiment to make the development loop faster or its feedback
sooner, what it measured, and whether it's still in use. This is the one
place for that. [DEVELOPMENT.md](../DEVELOPMENT.md#keeping-the-loop-fast)
holds the rules and how-tos; its slow-spot list holds problems no one has
tried yet. Issues about the loop carry the `dev-loop` label
(`gh issue list --label dev-loop --state all`).

**When you try something, add it here in the same commit:**
- **active** if it landed and is in use;
- **dropped** if it was tried and didn't pay, with its numbers, so no one
  tries it blind again;
- **superseded** if something later replaced it, naming what did.

Measure before and after, on the same machine and under the same load, with
`bun run check:fast --all --profile` for core use, and `bun run task` for a
task's wall time against its check time (the ledger of task times is #882).

## Active

| Speed-up | Before | After | How | Where |
|---|---|---|---|---|
| macOS's scan off for the app that runs the commands | 300 ms per new binary, 6 s for 20 | 2.5 ms each, 0.05 s for 20 | Exempted the binary just below `launchd` in Developer Tools (the M1's corpus went from about 10.5 min to 144 s) | #879 |
| num-traits in the crate corpus (#865) | 172 s at 11.6 GB a `rust-js` since #847, its `bounds.js` 240 MB; `check:fast --all`'s crate corpus killed at its cap | 9 s, `bounds.js` 253 KB | Drop glue's `catch` runs the later parts flat, as Rust's cleanup does, not each in its own `try` again: its JS grows as the square of the parts, not twice a part | f58ce2245, #865 |
| `test/tidy-stages.test.ts` (#808) | 109 s alone; timed out at 60 s under load | 31–38 s | Each case's variants compiled at once, and one folder per case | aeb9f560d, #808 |
| `check:fast --all`'s jobs through a pool of the cores (#857) | 3,530 s on the M1's 8 cores, its jobs all at once, a load of 17–19, a test timed out at 180 s; corpus parts 9–13 min, the matrix's slice 11–12 min | 2,877 s and 3,390 s, twice, no test timed out, a load near the cores; corpus parts 2.3–2.6 min, the matrix's slice 2.6 min, the conformance table's 1.6 min, every other test file 7.7 min | `run()` starts each job once the cores it keeps busy, its `weight`, are free, the longest usual first; each job gets `RUST_JS_LOAD_SCALE`, which `underLoad` reads in place of the load as a file starts; a job's cap is twice its usual at least | 181f49b75, #857 |
| `check:fast --all`'s cores kept busy (pool1, 2026-11-07) | On the 12-core M6, with no change, 3 min 55 s, the cores 63% busy, its last minute 35% and a third of its time under half (6 min 0 s at 56% where the corpus ran again); with an iterator change, 6 min 33 s at 76%, its last minutes 56%, 58% and 31% | With no change, 2 min 56 s and 2 min 57 s three runs in a row, the cores 93–94% busy by the same sampler, under half only as it starts; with the iterator change, 5 min 50 s at 91%; no test timed out | Each test file a job of the pool's, the longest first (one job of `bun test --parallel=6` began after the corpus and ran on alone); the browsers' files beside the rest, not after; the corpus in 12 parts, not 6; each job held at the cores its last runs measured, its CPU seconds over its own, not a guessed weight (a corpus part said two and kept one, the browsers' files said twelve and kept 3.6), more by the load only where every core was busy, as macOS's load reads 16 on 12 cores 86% busy. Half the M6's cores are efficiency cores, so the cores busier buy less time than they show. `--profile` says each minute's and each job's | ebf19a641, #881 |
| `architecture.test.ts`'s "each owner and question" (#832) | 21–30 s under load; timed out at 30 s | 0.27 s | One pass over the source to find the defined items, instead of one regex per name | d967a5dcb, #832 |
| Both corpus builds, default and `RUST_JS_LIBRARIES=all` | about 7 min, one after the other (8 cores) | about 4 min | Run as one set of parts | m1-lisa, 2026-11-06 |
| A gate before a push (m6-lily's) | about 14.5 min, its steps one after another, mutations `--jobs=4` | 12.2 min with nearly four times the mutations | `check:fast`, rustc's tests, the suites and mutations at once after the build, mutations `--jobs=10 --tests=12` | m6-lily's gate4, 2026-11-06 |
| One agent per machine | load 50–60 on 12 cores, each step 3–5× slower | each step at its usual | Several agents on one machine stopped | 911be1ab8, #878 |
| The iterator matrix and conformance rows a change touches (m6-lily, #834) | a full `ITERATOR_MATRIX=all` run, about 25 min on 12 parts | about 2 min for every batch with a `step_by` row (505 batches, 17,367 rows) | Only the batches whose rows touch the changed adapters, whole batches so each row is judged as in a full run, native and both builds 12-wide (the prototype of #818) | ba2dc8b83, #834 |
| `bun scripts/crate-corpus.ts --check`, the crate corpus (#828) | 102 s with rust-js's outputs to make again, one crate at a time, at a load near 1 on 12 cores | 33 s; 35 s from an empty `target/crate-corpus` | Half the cores' crates measured at once, each in its own project, printed in the table's order (2026-11-06) | b3f63df92, #880 |
| What an iterator change runs of the matrix before it lands (#818) | the fixed slices alone, or every one of 128,766 rows, about 17 min on 12 cores, which only the nightly ran, so three fix-on-fix regressions in a day reached `main` | the rows its hunks name, a typical change's (#834's) 14,700 in 111 s in 12 parts; with `pairs`, of a change to what every adapter shares, about 5 min (#810's reintroduced, caught) | `scripts/iteratorRows.ts` picks each hunk's adapters by its identifiers, `ITERATOR_MATRIX='/<regex>/'` runs just those rows, rebatched; `check:fast` prints hunk, names and rows | e6ffee46a, f5c1f51b5, #818 |
| The rows an iterator change runs, on any machine (#857's measure) | 43–55 min on the M1's 8 cores in 4 parts, the whole tail of `check:fast --all`: a budget of 16,000 and 5,000 rows sized for 12 cores | about 4 min there for a typical change (#834's: 983 and 844 rows), 2–3 min on the M3 Max in 12 parts (#834's 10,593 and 4,741, 111 s) | `scripts/iteratorRows.ts`' `budgetOf`: the rows its parts run in 150 s and 40 s at the machine's own rows a second per part, the median of its last runs' (`target/iterator-rows-rates.json`, the M3 Max's until it has some), and the shared sample a fifth of it; a removal's rows still each run (#823) | e79cbe108 |

Also in use, with no single before and after:
| Practice | Where | Why |
|---|---|---|
| One build of a workspace, not one per crate | d21d52c8d, ADR 1068 | A loop of builds ran one core at a time |
| sccache shared by the worktrees | DEVELOPMENT.md | Worktrees reuse each other's compiled dependencies |
| check:fast's jobs run their own hard link of the compiler | 5f927708b | A build in the checkout during a run can't change it; a hard link isn't scanned again |
| `check:fast --all` runs what a push's check runs that can run here | 1b65f3b1d, #732, ADR 1093 | What passes it passes CI's tests |
| check:fast refuses to start mid-rebase | 5d058e285 | A build of a half-applied tree sent agents after a red main that wasn't |
| Every test's timeout of three minutes or less is `underLoad(n)` | 545171f25 | Fixed timeouts failed under load |
| The determinism net: each case compiled again in its own process | b3416b692, #866 | Same input, same output; caches and snapshots depend on it |
| A task's time and its checks' share, `bun run task` | a78634237, #882 | So a speed-up's return shows in agents' tasks |

## Dropped

| Tried | Measured | Why dropped | Where |
|---|---|---|---|
| Several agents on one machine, sharing cores through a lease tool | 8 agents: load 50–60 on 12 cores, each step 3–5× slower. Leased 6/3 against 12/12: both pairs finished at the same time | Splitting cores gained nothing; only capping concurrent jobs did | #878 |
| A session-local CPU gate script, `load-gate.sh` | Half the agents skipped it, voluntarily | One agent per machine, and check:fast's own pool | #857 |

## Superseded

| Was | Replaced by | Where |
|---|---|---|
| The browsers' test files run after every other job | Run alongside the rest, now that the pool holds the load near the cores | ebf19a641, ADR 1093 amended |
| `check:fast` starting every job at once, timeouts scaled by the load | The pool sized to the cores, with each job's measured core use | 181f49b75, ebf19a641 |
| One "every other test file" job (`bun test --parallel=6`) | Each test file a job of the pool, longest first | ebf19a641 |
| Corpus parts at one for each two cores | One part per core under the pool (`pooledParts`) | ebf19a641 |
| A fixed rows budget of 16,000 + 5,000 sized for 12 cores | A budget from each machine's own measured rate | e79cbe108 |

## In progress

| Experiment | Status | Where |
|---|---|---|
| Cache each compile to JS by its exact inputs, bounded | First numbers: no-change 176 s with no cache, 176–177 s with a warm one. No gain yet, still being profiled | #874 |
| Cache native Rust results by source | Not started | #875 |
| Trim the iterator rows and std-structs jobs, about 1,270 of 3,500 CPU-s | Not started | #876 |
| Plain `check:fast` runs only the test files a change can affect | Not started | #877 |
| React's port regen on all cores, with unchanged packages cached (504 s at about 265% CPU) | Not started | #884 |
| A form-only mutation is caught through the corpus snapshots | Not started | #864 |
