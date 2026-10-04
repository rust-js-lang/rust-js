// Mutations of src/runtime/div_ceil.js (ADR 0093).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "div-ceil-floor",
    breaks: "`div_ceil` rounds down",
    file: "src/runtime/div_ceil.js",
    find: "  return Math.trunc(a / b) + (a % b > 0 ? 1 : 0);",
    replace: "  return Math.trunc(a / b);",
    tests: ["test/corpus.test.ts", "-t", "number_methods"],
  },
];
