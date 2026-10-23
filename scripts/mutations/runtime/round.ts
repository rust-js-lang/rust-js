// Mutations of src/runtime/round.js (ADR 0093).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "round-ties-even-up",
    breaks: "`round_ties_even()` takes a tie up, as JS's `Math.round` does, `2.5` to `3`",
    file: "src/runtime/round.js",
    find: "  return r - x === 0.5 && r % 2 !== 0 ? r - 1 : r;\n",
    replace: "  return r;\n",
    tests: ["test/corpus.test.ts", "-t", "^round_ties.rs$"],
  },
];
