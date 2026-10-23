// Mutations of src/runtime/clamp.js (ADR 0093).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "clamp-by-max-first",
    breaks: "`clamp` of text past both bounds takes the lower as the upper",
    file: "src/runtime/clamp.js",
    find: "  return cmp(x, min) < 0 ? min : cmp(x, max) > 0 ? max : x;\n",
    replace: "  return cmp(x, min) < 0 ? min : cmp(x, max) > 0 ? min : x;\n",
    tests: ["test/corpus.test.ts", "-t", "^(clamp_ord|clamp_char_bounds)\\.rs$"],
  },
];
