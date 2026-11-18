// Mutations of std/'s freshness check (scripts/std-fresh.ts), ADR 0093.
import type { Mutation } from "../mutations";

export const mutations: Mutation[] = [
  {
    name: "std-fresh-blind",
    breaks: "std/ that differs from what's committed is said fresh, so a runtime change is tested against the old code and passes (#1971)",
    file: "scripts/std-fresh.ts",
    find: '  return status.stdout.split("\\n").filter((line) => line.trim() !== "");\n',
    replace: "  return [];\n",
    tests: ["test/std-fresh.test.ts"],
  },
  {
    name: "std-fresh-of-the-whole-tree",
    breaks: "a change outside std/ is taken for std/ that's stale, so every push with another change is refused",
    file: "scripts/std-fresh.ts",
    find: '  const status = spawnSync("git", ["status", "--porcelain", "--", "std"], { cwd: dir, encoding: "utf8" });\n',
    replace: '  const status = spawnSync("git", ["status", "--porcelain"], { cwd: dir, encoding: "utf8" });\n',
    tests: ["test/std-fresh.test.ts"],
  },
];
