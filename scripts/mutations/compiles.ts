// Mutations of the compile cache (test/compiles.ts): what a kept compile is counted by.
import type { Mutation } from "../mutations";

export const mutations: Mutation[] = [
  {
    name: "kept-compile-counted-by-lengths",
    breaks: "a kept compile is counted by its files' lengths, not the blocks it takes, `entry.json`'s left out, so a store `prune` holds to its 2 GiB cap takes 5 on the disk (ADR 1118, amended)",
    file: "test/compiles.ts",
    find: "  const bytes = bytesOf(building);\n",
    replace: "  const bytes = files.reduce((sum, file) => sum + file.length, 0);\n",
    tests: ["test/compiles.test.ts", "-t", "counted by its blocks on the disk"],
  },
  {
    name: "building-uncounted",
    breaks: "what a writer is building, or a writer that ended left, isn't counted in the store's size, so a store at its cap holds more on the disk than its cap (#2107)",
    file: "test/compiles.ts",
    find: "  const size = kept.size + building;\n",
    replace: "  const size = kept.size;\n",
    tests: ["test/compiles.test.ts", "-t", "a compile being built counts in the store's size"],
  },
  {
    name: "building-counted-before-cleanup",
    breaks: "a writer's leftover an hour old is counted in the store's size by the prune that removes it, so the size says more than's on the disk (#2107)",
    file: "test/compiles.ts",
    find: "  const all = entries(store);\n  const building = buildingBytes(store);\n",
    replace: "  const building = buildingBytes(store);\n  const all = entries(store);\n",
    tests: ["test/compiles.test.ts", "-t", "a compile being built counts in the store's size"],
  },
];
