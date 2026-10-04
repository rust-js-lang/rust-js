// Mutations of src/runtime/saturating_pow.js (ADR 0093).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "saturating-pow-sign",
    breaks: "a negative base to an odd power saturates at the maximum, not the minimum",
    file: "src/runtime/saturating_pow.js",
    find: "  return base < 0 && exp % 2 === 1 ? lo : hi;",
    replace: "  return hi;",
    tests: ["test/corpus.test.ts", "-t", "integer_families"],
  },
];
