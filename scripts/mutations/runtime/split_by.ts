// Mutations of src/runtime/split_by.js (ADR 0157).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "split-by-unlimited",
    breaks: "`splitn(2, p)` of a predicate splits at every match, not leaving the rest whole",
    file: "src/runtime/split_by.js",
    find: "    if (pieces.length < n && matches(c)) {\n",
    replace: "    if (matches(c)) {\n",
    tests: ["test/corpus.test.ts", "-t", "^splitn_by.rs$"],
  },
];
