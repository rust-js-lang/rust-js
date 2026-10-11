// Mutations of src/runtime/range_next_back.js (ADR 0364).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "range-back-char-as-number",
    breaks: "a `char` range is stepped from its end as a number",
    file: "src/runtime/range_next_back.js",
    find: "  if (typeof range.start === \"string\") {\n",
    replace: "  if (false) {\n",
    tests: ["test/mir.test.ts","-t","char_ranges_stepped"],
  },
];
