// Mutations of src/runtime/sorted_entries.js (ADR 0093).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "sorted-entries-sorts-ord-map",
    breaks: "a B-tree of its keys' own order is sorted again as it's walked, asking `cmp`, where std's walk asks nothing (#911)",
    file: "src/runtime/sorted_entries.js",
    find: "  if (map.found !== undefined) return Array.from(map);\n",
    replace: "",
    tests: ["test/corpus.test.ts",  "-t",  "^btree_search_order.rs$"],
  },
];
