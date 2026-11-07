// Mutations of src/runtime/add.js (ADR 0467).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "add-item-undropped",
    breaks: "a set's `insert` of an item equal to one there drops neither",
    file: "src/runtime/add.js",
    find: "  if (found.there) drop?.(item);\n",
    replace: "  if (found.there);\n",
    tests: ["test/corpus.test.ts", "-t", "^map_key_drops.rs$"],
  },
  {
    name: "add-given-leaked",
    breaks: "a B-tree set's `insert` whose search panics leaks the item it was given, where std drops it (#911)",
    file: "src/runtime/add.js",
    find: "    drop && (() => drop(item)),\n",
    replace: "",
    tests: ["test/corpus.test.ts",  "-t",  "^btree_search_order.rs$"],
  },
];
