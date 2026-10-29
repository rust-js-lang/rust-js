// Mutations of src/runtime/key_set.js (ADR 0093).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "key-set-parts-unread",
    breaks: "a set whose items hold a fiber keys each by `$key` of it, the fiber's fields (#398)",
    file: "src/runtime/key_set.js",
    find: "      return $key(parts(item));\n",
    replace: "      return $key(item);\n",
    tests: ["test/corpus.test.ts", "-t", "^identity_inside_keys.rs$"],
  },
];
