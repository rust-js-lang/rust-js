// Mutations of src/runtime/resize_with.js (ADR 0098).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "resize-with-shrink-drops-none",
    breaks: "`resize_with` shortening a vector of items with a destructor drops none of what it takes out",
    file: "src/runtime/resize_with.js",
    find: "    $truncate(v, length, drop);\n",
    replace: "    v.length = length;\n",
    tests: ["test/corpus.test.ts", "-t", "^vec_destructors.rs$"],
  },
];
