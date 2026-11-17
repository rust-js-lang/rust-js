// Mutations of serde's snapshots kept for the tests (test/serde-kept.ts): a sweep that skips what another run removed.
import type { Mutation } from "../mutations";

export const mutations: Mutation[] = [
  {
    name: "serde-kept-sweep-throws-on-gone",
    breaks: "serde's snapshot store's sweep throws on an entry another run's sweep removed since it was listed: crates.test.ts's ENOENT (2026-11-17)",
    file: "test/serde-kept.ts",
    find: "    const seen = statSync(at, { throwIfNoEntry: false });\n",
    replace: "    const seen = statSync(at);\n",
    tests: ["test/support.test.ts", "-t", "a store's sweep skips an entry gone since it was listed"],
  },
];
