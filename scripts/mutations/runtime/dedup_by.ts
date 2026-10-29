// Mutations of src/runtime/dedup_by.js (ADR 0098).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "dedup-by-drops-none",
    breaks: "`dedup_by` of items with a destructor drops none it takes out",
    file: "src/runtime/dedup_by.js",
    find: "        drop?.(item);\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^vec_destructors.rs$"],
  },
];
