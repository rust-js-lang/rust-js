# 1146. Concurrent checkouts share the machine

Status: accepted (2026-11-14)

## Context

Two checkouts of the repository, worktrees or a worktree beside the main
checkout, may run their checks and mutation runs on one machine at once.
Each sized itself as if it had the machine alone. Two `check:fast --all`
runs started together both passed, but:

- **The cores were oversubscribed.** Each pool took every core: the load
  averaged 3.5 times the cores and peaked at 4.8 times them, and each check
  took 507 s and 519 s, against about 300 s alone.
- **Job times could be lost.** Two runs ending together each read the
  machine's job times, added theirs and wrote what they had, so one's runs
  were lost; a test of two writers kept 12 and 36 of each one's 40.
- **One checkout's pruning reached into another's live stores.**
  `disk.ts --prune` bounded every checkout's `crate-corpus`, `crater` and
  `mutants`, which could remove a project the other run was building.
- **Two mutation runs in one checkout built into one `target/mutants`,**
  each the other's mutant.
- **Vitest's browser server listened on 63315 by default,** moving to the
  next port if that one was taken. A concurrent run racing for the same
  port can leave a browser waiting on the wrong server; one did, timing out
  at 60 s.

## Decision

- **Leases** (`scripts/leases.ts`). A check or mutation run that sizes itself
  by the machine holds a lease in the machine's store while it runs. Its
  share is the machine's cores and memory divided by the live leases, its
  own among them (`fairShare`). A check's pool asks for its share again,
  at most once a second, as jobs start, and clamps each job's measured cores
  to it. A share counts runs, not the load, so two pools settle at half
  each and neither swings with the other's load; when one ends, the other
  takes the whole at its next job. A lease whose process is gone counts for
  nothing and is removed. What a pool starts inherits its lease
  (`RUST_JS_LEASED`) and holds none of its own.
- **Locks** (`acquire`, `withLock`): a directory made atomically, its owner a
  process. One whose owner's process is gone is taken, as is one with no
  owner named 5 s after it was made.
  - The job times' read, change and write is done under the store's lock;
    past its wait, nothing is written, the times being a guide.
  - A checkout's `target/mutants` is one mutation run's at a time; another
    waits, and says so.
- **Pruning** bounds another checkout's stores only while no live lease is
  in it. They're counted in the budget all the same.
- **Ports:** Vitest's browser server takes a port the system gives
  (`api.port: 0`). Every other server already did: Vite's and Playwright's
  through `port: 0`, Next's through a probe.
- **The pool backfills** (#1762): when the next job in its longest-first
  queue doesn't fit the room left, the first one after it that does starts,
  so free cores or memory aren't left idle. When nothing runs, the first
  job starts alone, as one heavier than the machine always has.

## Consequences

Two `check:fast --all` runs started together both pass, in 497 s and 511 s,
at a load that averaged 1.6 times the cores and peaked at 2 times them: half
the load, at the same wall times. One alone, backfilled, took 268 s against
its usual 322 s. docs/dev-loop.md has the times.
A run alone keeps the whole machine. A lease is per process, not per
checkout, so two runs in one checkout each take their share as well.

## Amendment: tracked source-owner mutations (#1848)

[ADR 1154](1154-source-mutations-run-in-isolated-workspaces.md) extends the
existing runner to isolated ECMAScript/TypeScript module inputs, with actual
load evidence, unchanged-source controls, per-workspace package aliases and
the same lease, lock and target isolation. Generator chains use existing
build workers; pure module tests use existing test slots.

Amended (#1767): a mutation run took its share of the cores (`ownShare`) but sized its builds and its tests' memory by the whole budget, so beside another run it planned for memory its neighbour held. Both now come from one part of the machine, `ownPart`: `ownShare` of the cores and `ownMemory` of the memory budget, which `compilerBuildsAsked` and `testsMemory` read.

## Amendment: one heavy run at a time (Tu's rule, 2026-11-24)

Where agents share a machine, as the Mac mini's and the M5 Pro's coordinators each run two, a fair share of the cores still slows every run: two checks at half the machine each take about twice as long as one alone, so neither lands sooner. The rule is now **one heavy run at a time on a machine**, built on the lease:

- **A lease is taken with the machine's turn** (`turnLock`, a lock beside the leases' store). A check, a push (its whole run, its check and mutants inheriting it), a mutation run or a sweep built on them waits for the turn while another holds it, and says so each minute with the holder's process. What other agents do meanwhile is medium or light: reading, editing, single test files, small builds.
- **A lane's run (`RUST_JS_LANE=1`), background work, starts only where no run holds a lease,** and takes no turn. A run started after it goes on at once beside it. The lane runs at `nice -n 10`, and `sharers` doesn't count it against the run's share, while the lane counts the run.
- **What a run starts inherits its lease,** `RUST_JS_LEASED` naming the holder's process, which `sharers` doesn't count again.

The fair share stays for what still overlaps: a run beside a lane, or a lane beside a lane.

Measured on the M5 Pro, 18 cores, quiet and warm (2026-11-24). Each medium loop was a `cargo build` after one file's change plus two single test files, about 17 s:

| Run | Wall time |
|---|---|
| `check:fast --all` alone | 180.4 s and 189.9 s (mean 185.2 s) |
| Beside a loop of medium work at `nice -n 10` | 191.3 s (+3.3%), 11 loops done beside it |
| At cores − 2, beside the same loop | 204.7 s (+10.5%), 13 loops |

So the heavy run keeps the machine's whole width, and what runs beside it runs niced. Leaving cores spare costs the heavy run more than it gives the work beside it.

## Amendment: the turn has an order, and a push goes first (#2516, 2026-11-24)

The turn had no order: each waiter polled every 25 ms, and whichever polled
first after it was let go took it. On 2026-11-24 the hulk agent's push of a
fix for a red `main` waited 34 minutes behind a subagent's `--retire`, then
behind that subagent's next one, started in the moment between.

- **Pushes go next.** A run waiting for the turn writes what it is and
  since when in the waiters' store beside the leases (`waiterStore`), and
  takes the turn only where it's next (`nextWaiter`): a push first, then
  the longest waiting. A waiter whose process is gone counts for nothing.
- **Long runs yield between batches.** A mutation run lets the turn go
  where a push waits (`pushWaits`, `yieldToPush`): its workers take no new
  batch, those running and their tests end, the turn and the lease go to
  the push, and the run waits for them again as it first did, since then,
  so it's next after the push and before a run that began to wait later.
  A retirement does so between names, the name before judged first.
  Neither stops mid-batch, so no work is lost.
- **Measured** by the push's line: its wait for the turn is its first phase,
  `turn <s>`.

Held by test/leases.test.ts: "a push waiting goes before a run that waited
longer, and the rest by how long each waited", and "a run of three batches
lets a push have the turn between two, and finishes its third after it",
each of a run calling `yieldToPush` itself, a mutation run's pool
(`runYielding`, which the runner's pool is) and a retirement's names
(`yieldBetween`). Mutations `turn-order-ignored`, `push-not-first`,
`push-yield-never-made`, `mutation-run-never-yields`, `names-never-yield`,
`retire-never-yields`, `mutation-pool-unyielding`;
`heavy-runs-at-once` and `lock-absent-taken-for-stale` retargeted.
