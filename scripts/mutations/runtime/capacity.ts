// Mutations of src/runtime/capacity.js (ADR 0419).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "shrink-to-unshrunk",
    breaks: "`shrink_to_fit()` leaves its room",
    file: "src/runtime/capacity.js",
    find: "  $capacities.set(v, Math.max(v.length, Math.min($capacity(v), min)));\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^vec_capacity.rs$"],
  },
];
