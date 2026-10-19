// Mutations of src/runtime/reduce.js (ADR 0479).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "reduce-of-none-js",
    breaks: "`reduce` of no items is JS's, which throws",
    file: "src/runtime/reduce.js",
    find: "  if (first.done) {\n    return undefined;\n  }\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^iterator_reduce.rs$"],
  },
];
