// Mutations of src/runtime/is_sorted.js (ADR 0093).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "is-sorted-nan-sorted",
    breaks: "`is_sorted()` of floats takes a NaN as in order, `last > item` being false of it",
    file: "src/runtime/is_sorted.js",
    find: "      if (cmp ? order !== -1 && order !== 0 : !(last <= item)) return false;\n",
    replace: "      if (cmp ? order !== -1 && order !== 0 : last > item) return false;\n",
    tests: ["test/corpus.test.ts", "-t", "^is_sorted_items.rs$"],
  },
  {
    name: "is-sorted-never-unsorted",
    breaks: "`is_sorted()` never finds a pair out of order, and steps every item of an iterator",
    file: "src/runtime/is_sorted.js",
    find: "      if (cmp ? order !== -1 && order !== 0 : !(last <= item)) return false;\n",
    replace: "      if (cmp ? order !== -1 && order !== 0 : !(last <= item)) first = first;\n",
    tests: ["test/corpus.test.ts", "-t", "^is_sorted_items.rs$"],
  },
];
