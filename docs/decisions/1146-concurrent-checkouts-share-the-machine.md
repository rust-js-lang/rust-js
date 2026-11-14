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
