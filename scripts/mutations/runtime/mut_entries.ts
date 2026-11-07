// Mutations of src/runtime/mut_entries.js (ADR 0093).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "mut-entries-by-search",
    breaks: "a B-tree's handles on its numbers search for their key at each read and write, asking `cmp` where std's walk asks nothing (#911)",
    file: "src/runtime/mut_entries.js",
    find: "  if (m.entryHandles !== undefined) return m.entryHandles(entries);\n",
    replace: "",
    tests: ["test/corpus.test.ts",  "-t",  "^btree_search_order.rs$"],
  },
];
