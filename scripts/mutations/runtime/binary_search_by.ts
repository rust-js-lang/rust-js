// Mutations of src/runtime/binary_search_by.js (ADR 0093).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "binary-search-by-moves-on-less",
    breaks: "`binary_search_by` looks right of an item that's `Less` than what's sought, left of a `Greater` one",
    file: "src/runtime/binary_search_by.js",
    find: "    if (f(items[mid]) !== 1) {",
    replace: "    if (f(items[mid]) !== -1) {",
    tests: ["test/corpus.test.ts", "-t", "collection_methods"],
  },
];
