// Mutations of src/runtime/clamp_float.js (ADR 0093).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "clamp-float-nan-bound",
    breaks: "a NaN bound of a float's `clamp` isn't noticed",
    file: "src/runtime/clamp_float.js",
    find: "  if (!(min <= max)) {",
    replace: "  if (min > max) {",
    tests: ["test/corpus.test.ts", "-t", "float_clamp_nan"],
  },
];
