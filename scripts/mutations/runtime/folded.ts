// Mutations of src/runtime/folded.js (#162).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "folded-steps",
    breaks: "what std routes through `fold`, `last` and `collect` into a string or a hash map, steps the crate's iterator by `next`",
    file: "src/runtime/folded.js",
    find: "  return it\n    .fold([], (items, item) => {\n      items.push(item);\n      return items;\n    })\n    .values();\n",
    replace: "  return it.toArray().values();\n",
    tests: ["test/corpus.test.ts",  "-t",  "^own_fold_and_nth.rs$"],
  },
];
