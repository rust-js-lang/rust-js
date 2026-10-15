// Mutations of src/runtime/some_at.js (ADR 0051).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "some-get-unboxed",
    breaks: "`get` of a map's value that looks like `None`, `None`, gives `None`",
    file: "src/runtime/some_at.js",
    find: "  return map.has(key) ? $some(map.get(key)) : undefined;\n",
    replace: "  return map.get(key);\n",
    tests: ["test/corpus.test.ts", "-t", "^map_values_like_none.rs$"],
  },
];
