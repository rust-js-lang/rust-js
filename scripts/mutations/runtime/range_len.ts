// Mutations of src/runtime/range_len.js (ADR 0093).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "range-len-past-end",
    breaks: "a range's `len()` past its end is negative, not zero",
    file: "src/runtime/range_len.js",
    find: "  return range.start < range.end ? range.end - range.start : 0;\n",
    replace: "  return range.end - range.start;\n",
    tests: ["test/corpus.test.ts", "-t", "^exact_size_len.rs$"],
  },
];
