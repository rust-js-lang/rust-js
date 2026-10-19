// Mutations of src/runtime/remove.js (ADR 0051).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "remove-old-unboxed",
    breaks: "a map's `remove` of what it had that looks like `None`, `()`, gives `None`",
    file: "src/runtime/remove.js",
    find: "  return had ? $some(old) : undefined;\n",
    replace: "  return old;\n",
    tests: ["test/corpus.test.ts", "-t", "^map_values_like_none.rs$"],
  },
  {
    name: "remove-key-undropped",
    breaks: "`remove` of a key with a destructor drops none",
    file: "src/runtime/remove.js",
    find: "  if (had) dropKey?.(held);\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^map_key_drops.rs$"],
  },
];
