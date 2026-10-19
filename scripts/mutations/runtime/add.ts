// Mutations of src/runtime/add.js (ADR 0467).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "add-item-undropped",
    breaks: "a set's `insert` of an item equal to one there drops neither",
    file: "src/runtime/add.js",
    find: "  if (!added) drop?.(item);\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^map_key_drops.rs$"],
  },
];
