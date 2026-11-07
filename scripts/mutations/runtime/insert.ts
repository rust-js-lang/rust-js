// Mutations of src/runtime/insert.js (ADR 0051).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "insert-old-unboxed",
    breaks: "a map's `insert` of what it had that looks like `None`, `()`, gives `None`",
    file: "src/runtime/insert.js",
    find: "  return found.there ? $some(found.value) : undefined;\n",
    replace: "  return found.value;\n",
    tests: ["test/corpus.test.ts", "-t", "^map_values_like_none.rs$"],
  },
  {
    name: "insert-key-undropped",
    breaks: "`insert` of a key equal to one there keeps the old and drops neither",
    file: "src/runtime/insert.js",
    find: "  if (found.there) dropKey?.(key);\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^map_key_drops.rs$"],
  },
  {
    name: "insert-given-key-leaked",
    breaks: "a B-tree's `insert` whose search panics leaks the key it was given, where std drops it (#911)",
    file: "src/runtime/insert.js",
    find: "    dropKey && (() => dropKey(key)),\n",
    replace: "",
    tests: ["test/corpus.test.ts",  "-t",  "^btree_search_order.rs$"],
  },
  {
    name: "insert-given-value-leaked",
    breaks: "a B-tree's `insert` whose search panics leaks the value it was given, where std drops it after the key (#911)",
    file: "src/runtime/insert.js",
    find: "    dropValue && (() => dropValue(value)),\n",
    replace: "",
    tests: ["test/corpus.test.ts",  "-t",  "^btree_search_order.rs$"],
  },
];
