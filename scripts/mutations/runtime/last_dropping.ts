// Mutations of src/runtime/last_dropping.js (ADR 0098).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "last-dropping-drops-none",
    breaks: "`last()` of items with a destructor drops none it passes",
    file: "src/runtime/last_dropping.js",
    find: "    if (seen) drop(last);\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^vec_destructors.rs$"],
  },
];
