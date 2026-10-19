// Mutations of src/runtime/into_iter.js (ADR 0475).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "into-iter-of-array-js",
    breaks: "`$intoIter` of an array is JS's iterator of it, which knows nothing of where it is",
    file: "src/runtime/into_iter.js",
    find: "  return Array.isArray(items) ? $iter(items) : Iterator.from(items);\n",
    replace: "  return Iterator.from(items);\n",
    tests: ["test/corpus.test.ts", "-t", "^generic_into_iter_values.rs$"],
  },
];
