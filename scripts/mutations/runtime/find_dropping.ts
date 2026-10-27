// Mutations of src/runtime/find_dropping.js (ADR 0093).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "find-dropping-keeps-rejected",
    breaks: "`find` of items with a destructor drops none it rejects",
    file: "src/runtime/find_dropping.js",
    find: "      drop?.(x);\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^picked_items_drop.rs$"],
  },
];
