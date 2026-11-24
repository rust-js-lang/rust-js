// Leases and locks (scripts/leases.ts): whether a lock is held, which
// `acquire` waits on and the pruner leaves a mutation run's store alone
// by (#2272).
import type { Mutation } from "../mutations";

export const mutations: Mutation[] = [
  {
    name: "heavy-runs-at-once",
    breaks: "a second heavy run on the machine starts beside the first, not after it, the agents' runs fighting for the cores (Tu's rule, 2026-11-24)",
    file: "scripts/leases.ts",
    find: "    releaseTurn = acquire(turnLock(store), {",
    replace: "    releaseTurn = undefined && acquire(turnLock(store), {",
    tests: ["test/leases.test.ts", "-t", "one heavy run at a time"],
  },
  {
    name: "lane-starts-beside-a-run",
    breaks: "a lane's run starts while a heavy run holds the machine, taking its cores (Tu's rule, 2026-11-24)",
    file: "scripts/leases.ts",
    find: "    while (liveLeases(store).some((lease) => lease.pid !== process.pid) || lockState(turnLock(store)) === \"held\") {",
    replace: "    while (false) {",
    tests: ["test/leases.test.ts", "-t", "one heavy run at a time"],
  },
  {
    name: "lane-counted-against-a-run",
    breaks: "a run started beside a lane takes half the machine, the lane's lower priority counted as an equal share",
    file: "scripts/leases.ts",
    find: "!own.has(lease.pid) && (lane || !lease.lane)",
    replace: "!own.has(lease.pid)",
    tests: ["test/leases.test.ts", "-t", "one heavy run at a time"],
  },
  {
    name: "lock-held-past-its-owner",
    breaks: "a lock whose owner's process is gone is held still, so `acquire` waits on a killed run's lock for hours and the pruner never bounds its store again (#2272)",
    file: "scripts/leases.ts",
    find: "  const live = held !== undefined && Number.isFinite(held) && held > 0 ? alive(held) : Date.now() - made <= 5_000;\n",
    replace: "  const live = held !== undefined && Number.isFinite(held) && held > 0 ? true : Date.now() - made <= 5_000;\n",
    tests: ["test/leases.test.ts"],
  },
  {
    name: "lock-absent-taken-for-stale",
    breaks: "a lock found absent, its holder done, is removed as a stale one, so what another run made in that instant goes and both hold it, two job-time writers each writing what it read (#2283)",
    file: "scripts/leases.ts",
    find: "      if (state === \"absent\") continue;\n      if (state === \"stale\") {\n",
    replace: "      if (state !== \"held\") {\n",
    tests: ["test/leases.test.ts", "-t", "found absent"],
  },
  {
    name: "lock-wait-said-once",
    breaks: "a run waiting on a lock says so once, and its wait, which sleeps the process, is unseen after: a mutation run waited 50 minutes saying nothing more (#2418)",
    file: "scripts/leases.ts",
    find: "      } else if (waiting && said !== undefined && Date.now() - said >= every) {",
    replace: "      } else if (waiting && said !== undefined && Date.now() - said >= every * 1000) {",
    tests: ["test/leases.test.ts", "-t", "a lock is one holder's at a time"],
  },
];
