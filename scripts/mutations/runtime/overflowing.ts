// Mutations of src/runtime/overflowing.js (ADR 0093).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "overflowing-never",
    breaks: "`overflowing_*` never says it overflowed",
    file: "src/runtime/overflowing.js",
    find: "  return [wrapped, exact < lo || exact > hi];",
    replace: "  return [wrapped, false];",
    tests: ["test/corpus.test.ts", "-t", "integer_families"],
  },
];
