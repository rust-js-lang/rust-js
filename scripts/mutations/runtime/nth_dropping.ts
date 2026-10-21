// Mutations of src/runtime/nth_dropping.js (ADR 0093).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "nth-dropping-keeps-skipped",
    breaks: "`nth` of items with a destructor drops none it skips",
    file: "src/runtime/nth_dropping.js",
    find: "    drop?.(item);\n    n--;\n",
    replace: "    n--;\n",
    tests: ["test/corpus.test.ts", "-t", "^picked_items_drop.rs$"],
  },
];
