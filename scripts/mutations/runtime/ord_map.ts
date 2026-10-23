// Mutations of src/runtime/ord_map.js (ADR 0404).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "ord-map-search-misses",
    breaks: "an `$OrdMap`'s binary search never finds a key, so it keeps two `cmp` finds `Equal`",
    file: "src/runtime/ord_map.js",
    find: "      if (order === 0) return [mid, true];\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^btree_own_ord.rs$"],
  },
  {
    name: "ord-map-insert-replaces-key",
    breaks: "an `$OrdMap`'s `insert` of a key it holds an equal one of replaces that key, where std keeps it",
    file: "src/runtime/ord_map.js",
    find: "      this.#entries[i][1] = value;\n",
    replace: "      this.#entries[i] = [key, value];\n",
    tests: ["test/corpus.test.ts", "-t", "^btree_own_ord.rs$"],
  },
  {
    name: "ord-set-add-replaces",
    breaks: "an `$OrdSet`'s `insert` of an item it holds an equal one of replaces that item, where std keeps it",
    file: "src/runtime/ord_map.js",
    find: "    if (!this.#items.has(item)) {\n      this.#items.set(item, item);\n    }\n",
    replace: "    this.#items.delete(item);\n    this.#items.set(item, item);\n",
    tests: ["test/corpus.test.ts", "-t", "^btree_own_ord.rs$"],
  },
];
