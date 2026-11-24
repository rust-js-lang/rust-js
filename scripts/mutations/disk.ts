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
    find: "  const inside = stores.filter((store) => !store.outside);",
    replace: "  const inside = stores;",
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
  {
    name: "sccache-config-overwritten",
    breaks: "sccache's config a person wrote is written over with the disk's cap (#2505)",
    file: "scripts/disk.ts",
    find: "  if (had !== undefined && !had.startsWith(sccacheWritten)) return { own: true };\n",
    replace: "",
    tests: ["test/support.test.ts", "-t", "sccache's cap is the disk's"],
  },
  {
    name: "disk-budget-working-set-uncounted",
    breaks: "the stale kept past a budget take the room the working set uses, so the stores inside the checkouts keep that much past it",
    file,
    find: "  const left = holdKept(kept, { past: held - rest - workingBytes(kept, since), free, since, remove });",
    replace: "  const left = holdKept(kept, { past: held - rest, free, since, remove });",
    tests: ["test/support.test.ts", "-t", "a checkout past the budget keeps its working set"],
  },
  {
    name: "things-store-held-whole",
    breaks: "a store of things with no bound of its own, the check logs' or the temp root's, is held to its cap whole, its entries in use evicted as native-cache's were (#2344, #2483)",
    file,
    find: "  return holdKept(entriesOf(dir), { past, free, since });",
    replace: "  return evict(entriesOf(dir), { cap: past, free, floor: floorBytes });",
    tests: ["test/support.test.ts", "-t", "a store of things keeps its working set and what it keeps past it"],
  },
  {
    name: "things-store-cap-without-its-working-set",
    breaks: "`bun run disk` says a store of things' cap is what it keeps past its working set alone, so a store in use past it, std-reach's 438 MiB, reads as past its cap (#2483)",
    file,
    find: "  return { inUse, cap: inUse + store.past };",
    replace: "  return { inUse, cap: store.past };",
    tests: ["test/support.test.ts", "-t", "a store of things keeps its working set and what it keeps past it"],
  },
  {
    name: "budget-removes-whole-past-its-store",
    breaks: "the checkouts' budget removes a store's entry whole, not as its store does, so a crate's project a run began measuring since it was listed is removed under the run (#2562)",
    file: "scripts/disk.ts",
    find: "  const remove = (entry: Kept) => removers.get(entry)!(entry);",
    replace: "  const remove = (entry: Kept) => removeWhole(entry);",
    tests: ["test/support.test.ts", "-t", "a crate project being measured is never removed under its run"],
  },
  {
    name: "serde-kept-left-in-the-build",
    breaks: "serde's snapshots are counted as the build's, which nothing bounds or lists apart: 4.1 GiB of the M5 Pro's main checkout's 14.6 (#2483)",
    file: "scripts/disk.ts",
    find: '"npm-crates", "serde-kept"]);',
    replace: '"npm-crates"]);',
    tests: ["test/disk-owners.test.ts", "-t", "serde's snapshots are a store of their own, not the build's"],
  },
  {
    name: "serde-kept-store-unswept",
    breaks: "serde's snapshots' store has no bound of its own, so `bun run disk --prune` leaves those an hour unasked to the next test that asks for one",
    file: "scripts/disk.ts",
    find: '      bound: bounded ? () => void sweepSerdeKept(at("serde-kept")) : undefined,\n',
    replace: "",
    tests: ["test/disk-owners.test.ts", "-t", "serde's snapshots are a store of their own, not the build's"],
  },
];
