// Mutations of src/runtime/nth_dropping.js (ADR 0093).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "nth-dropping-drop-ungiven",
    breaks: "an `ownNth` iterator isn't given its items' drop, so a chain's `nth` past a compiled `fuse()`'s items never drops them (#285)",
    file: "src/runtime/nth_dropping.js",
    find: "  const step = it.nth(n, drop);\n",
    replace: "  const step = it.nth(n);\n",
    tests: ["test/corpus.test.ts", "-t", "^chain_sides_own_steps.rs$"],
  },
];
