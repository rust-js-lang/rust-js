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
];
