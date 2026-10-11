// Mutations of src/runtime/range_inclusive_next_back.js (ADR 0364).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "range-inclusive-next-back-before",
    breaks: "`next_back()` of an inclusive range skips its end",
    file: "src/runtime/range_inclusive_next_back.js",
    find: "range.end--",
    replace: "--range.end",
    tests: ["test/mir.test.ts","-t","range_inclusive_next_back"],
  },
  {
    name: "range-inclusive-back-char-as-number",
    breaks: "a `char` range is stepped from its end as a number, past its start",
    file: "src/runtime/range_inclusive_next_back.js",
    find: "  if (typeof range.start === \"string\") {\n",
    replace: "  if (false) {\n",
    tests: ["test/mir.test.ts","-t","char_ranges_stepped"],
  },
];
