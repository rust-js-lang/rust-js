// Leases and locks (scripts/leases.ts): whether a lock is held, which
// `acquire` waits on and the pruner leaves a mutation run's store alone
// by (#2272).
import type { Mutation } from "../mutations";

export const mutations: Mutation[] = [
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
];
