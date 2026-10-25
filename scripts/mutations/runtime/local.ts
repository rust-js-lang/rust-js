// Mutations of src/runtime/local.js (ADR 0093).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "local-made-at-once",
    breaks: "`$local` makes its thread-local where it's made, as its module loads",
    file: "src/runtime/local.js",
    find: "  let made = false;\n  let value;\n",
    replace: "  let made = true;\n  let value = init();\n",
    tests: ["test/corpus.test.ts", "-t", "^thread_local_made_where_used.rs$"],
  },
  {
    name: "local-made-though-it-panicked",
    breaks: "a thread-local whose initializer panicked is taken as made, so its next use reads nothing",
    file: "src/runtime/local.js",
    find: "    if (!made) {\n      value = init();\n      made = true;\n    }\n",
    replace: "    if (!made) {\n      made = true;\n      value = init();\n    }\n",
    tests: ["test/corpus.test.ts", "-t", "^thread_local_made_where_used.rs$"],
  },
];
