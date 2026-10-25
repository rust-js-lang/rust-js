// Mutations of src/runtime/key_map.js (ADR 0093).
import type { Mutation } from "../../mutations";

// Not mutated: which of two equal keys `insert` keeps, and which a drop is
// of. A key's `Eq` is derived (a key of a custom `PartialEq` is refused), so
// two equal keys print and drop alike, and no program can tell them apart.
export const mutations: Mutation[] = [
  {
    name: "key-map-has-by-identity",
    breaks: "`contains_key` asks for the key object itself, not its value (ADR 0121)",
    file: "src/runtime/key_map.js",
    find: "    return super.has(this.keyOf(key));\n",
    replace: "    return super.has(key);\n",
    tests: ["test/corpus.test.ts", "-t", "value_keys|generic_maps|map_eq|set_operators|value_key"],
  },
  {
    name: "key-map-delete-by-identity",
    breaks: "`remove` deletes the key object itself, not one of its value (ADR 0121)",
    file: "src/runtime/key_map.js",
    find: "    return super.delete(this.keyOf(key));\n",
    replace: "    return super.delete(key);\n",
    tests: ["test/corpus.test.ts", "-t", "value_keys|generic_maps|map_eq|set_operators|value_key"],
  },
  {
    name: "key-map-keys-are-strings",
    breaks: "a map's keys are the strings they're kept under, not the keys",
    file: "src/runtime/key_map.js",
    find: "    for (const [key] of super.values()) {\n      yield key;\n",
    replace: "    for (const key of super.keys()) {\n      yield key;\n",
    tests: ["test/corpus.test.ts", "-t", "value_key_iteration"],
  },
  {
    name: "key-map-made-empty",
    breaks: "a map made of entries holds none of them",
    file: "src/runtime/key_map.js",
    find: "    if (entries) {\n      for (const [key, value] of entries) {\n        this.set(key, value);\n      }\n    }\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "value_keys|generic_maps|map_eq|set_operators|value_key"],
  },
  {
    name: "key-map-for-each-key-first",
    breaks: "`forEach` gives each key before its value, where JS's gives the value first",
    file: "src/runtime/key_map.js",
    find: "      f.call(that, value, key, this);\n",
    replace: "      f.call(that, key, value, this);\n",
    tests: ["test/runtime-package.test.ts", "-t", "reads as a Map"],
  },
];
