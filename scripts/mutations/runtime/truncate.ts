// Mutations of src/runtime/truncate.js (ADR 0098).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "truncate-drops-none",
    breaks: "`truncate` and `clear` of items with a destructor drop none of what they take out",
    file: "src/runtime/truncate.js",
    find: "      for (const item of gone) drop(item);\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^vec_destructors.rs$"],
  },
];
