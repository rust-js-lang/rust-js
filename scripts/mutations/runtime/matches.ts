// Mutations of src/runtime/matches.js (ADR 0093).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "rmatches-by-from-start",
    breaks: "`rmatches` of a predicate gives its `char`s from the start",
    file: "src/runtime/matches.js",
    find: "  if (rev) chars.reverse();\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^matches_by_predicate.rs$"],
  },
];
