// What development keeps inside the checkouts, held to the budget together
// (scripts/disk.ts, #2163): a hold that never starts, or counts the builds
// as nothing, leaves the disk at the floor while each store is under its cap.
import type { Mutation } from "../mutations";

const file = "scripts/disk.ts";
const tests = ["test/support.test.ts", "-t", "the stores inside the checkouts are held to the budget together"];

export const mutations: Mutation[] = [
  {
    name: "disk-budget-never-held",
    breaks: "the stores past the budget are left as they are, each under its cap, as three worktrees' summed to 37.9 of 40 GiB",
    file,
    find: "  if (total <= held) return { removed: 0, size: total };",
    replace: "  if (total <= held || total > 0) return { removed: 0, size: total };",
    tests,
  },
  {
    name: "disk-builds-counted-as-nothing",
    breaks: "what no store may evict, a build, isn't counted in the total held, so the stores keep more than the budget leaves them",
    file,
    find: "  const rest = total - kept.reduce((n, entry) => n + entry.bytes, 0);",
    replace: "  const rest = 0;",
    tests,
  },
  {
    name: "disk-outside-stores-held-to-the-budget",
    breaks: "a store outside the checkouts, a tool's own, is evicted from to hold the checkouts' budget",
    file,
    find: "  const kept = stores.filter((store) => !store.outside).flatMap((store) => store.kept?.() ?? []);",
    replace: "  const kept = stores.flatMap((store) => store.kept?.() ?? []);",
    tests,
  },
  {
    name: "disk-over-budget-names-outside",
    breaks: "what's said past the budget names a store outside the checkouts among its largest, which pruning them can't help",
    file,
    find: "  const largest = stores\n    .filter((store) => !store.outside)\n",
    replace: "  const largest = stores\n",
    tests,
  },
  {
    name: "mutants-store-pruned-under-its-run",
    breaks: "a checkout's mutants store is pruned while a mutation run there holds its lock, so a retirement's each name's check:fast evicts the run's snapshot and the next name's build reads it gone (#2272)",
    file,
    find: "  const prunable = bounded && lockState(mutantsLock(tree)) !== \"held\";\n",
    replace: "  const prunable = bounded;\n",
    tests: ["test/leases.test.ts", "-t", "mutants store isn't pruned"],
  },
];
