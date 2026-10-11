// Mutations of src/runtime/range_next.js (ADR 0093).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "range-next-unmoved",
    breaks: "a `Range`'s `next()` gives its start but doesn't move it",
    file: "src/runtime/range_next.js",
    find: "  return range.start < range.end ? range.start++ : undefined;",
    replace: "  return range.start < range.end ? range.start : undefined;",
    tests: ["test/corpus.test.ts","-t","range_values"],
  },
  {
    name: "range-char-as-number",
    breaks: "a `char` range is stepped as a number",
    file: "src/runtime/range_next.js",
    find: "  if (typeof range.start === \"string\") {\n",
    replace: "  if (false) {\n",
    tests: ["test/mir.test.ts","-t","char_ranges_stepped"],
  },
];
