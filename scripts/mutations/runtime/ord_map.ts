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
  {
    name: "ord-map-appends",
    breaks: "an `$OrdMap`'s new key goes at its end, not where its order puts it",
    file: "src/runtime/ord_map.js",
    find: "      this.#entries.splice(i, 0, [key, value]);\n",
    replace: "      this.#entries.push([key, value]);\n",
    tests: ["test/corpus.test.ts", "-t", "^btree_own_ord.rs$|^btree_compare.rs$|^held_keys.rs$"],
  },
  {
    name: "ord-map-delete-missing-removes",
    breaks: "removing a key an `$OrdMap` doesn't hold removes the one where it would go",
    file: "src/runtime/ord_map.js",
    find: "    if (there) this.#entries.splice(i, 1);\n",
    replace: "    this.#entries.splice(i, 1);\n",
    tests: ["test/runtime-package.test.ts", "-t", "reads as a Map"],
  },
  {
    name: "ord-map-for-each-key-first",
    breaks: "an `$OrdMap`'s `forEach` gives each key before its value, where JS's `Map` gives the value first",
    file: "src/runtime/ord_map.js",
    find: "      f.call(that, value, key, this);\n",
    replace: "      f.call(that, key, value, this);\n",
    tests: ["test/runtime-package.test.ts", "-t", "reads as a Map"],
  },
  {
    name: "ord-map-size-of-nothing",
    breaks: "an `$OrdMap`'s `size` is the `Map`'s it extends, which holds nothing",
    file: "src/runtime/ord_map.js",
    find: "  get size() {\n    return this.#entries.length;\n  }\n",
    replace: "",
    tests: ["test/runtime-package.test.ts", "-t", "reads as a Map"],
  },
];
