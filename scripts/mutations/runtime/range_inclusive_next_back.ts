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
];
