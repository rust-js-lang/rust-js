// Mutations of src/runtime/range_inclusive_next.js (ADR 0364).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "range-inclusive-char-as-number",
    breaks: "a `char` range is stepped as a number, `\"a\" + 1`",
    file: "src/runtime/range_inclusive_next.js",
    find: "  if (typeof range.start === \"string\") {\n",
    replace: "  if (false) {\n",
    tests: ["test/corpus.test.ts","-t","range_into_set"],
  },
  {
    name: "range-inclusive-exhausted-unheld",
    breaks: "an inclusive range used up by a fold keeps no record of Rust's bounds, so it reads `4..=3`, not std's `3..=3 (exhausted)` (#414)",
    file: "src/runtime/range_inclusive_next.js",
    find: "  range.exhausted = { start, end };\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^range_inclusive_exhausted.rs$"],
  },
];
