// Mutations of src/runtime/or_insert.js (ADR 0321).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "or-insert-keeps-given",
    breaks: "`or_insert` of a value with a destructor where the key's there drops none of the one given",
    file: "src/runtime/or_insert.js",
    find: "    drop?.(value);\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^map_value_destructors.rs$"],
  },
  {
    name: "or-insert-key-undropped",
    breaks: "`entry(k).or_insert(v)` where an equal key's there drops the value, not the key",
    file: "src/runtime/or_insert.js",
    find: "    dropKey?.(key);\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^item_keyed_map.rs$"],
  },
];
