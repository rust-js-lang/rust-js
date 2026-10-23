// Mutations of src/runtime/or_insert_with.js (ADRs 0467, 0469).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "or-insert-with-unrun-kept",
    breaks: "`or_insert_with` of a key that's there drops none of its function, unrun",
    file: "src/runtime/or_insert_with.js",
    find: "    dropMake?.(make);\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^unrun_function_drops.rs$"],
  },
  {
    name: "or-insert-with-key-unkeyed",
    breaks: "`or_insert_with_key`'s function is given no key",
    file: "src/runtime/or_insert_with.js",
    find: "() => make(key)",
    replace: "() => make()",
    tests: ["test/corpus.test.ts", "-t", "^entry_or_insert_with_key.rs$"],
  },
];
