// Mutations of src/runtime/range_inclusive_next.js (ADR 0364).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "range-inclusive-char-as-number",
    breaks: "a `char` range is stepped as a number, `\"a\" + 1`",
    file: "src/runtime/range_inclusive_next.js",
    find: "  if (typeof range.start === \"string\") {\n",
    replace: "  if (false) {\n",
    tests: ["test/mir.test.ts","-t","range_into_set"],
  },
];
