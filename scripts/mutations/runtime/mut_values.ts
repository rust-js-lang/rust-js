// Mutations of src/runtime/mut_values.js.
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "mut-values-searched",
    breaks: "`values_mut` gives handles that search for each key, running a hashed key's `hash` and a B-tree's own `Ord`, where std's walk asks neither (#1362)",
    file: "src/runtime/mut_values.js",
    find: "  return $mutEntries(m, entries).map(([, handle]) => handle);\n",
    replace: "  return Array.from(entries, ([key]) => ({\n    get value() {\n      return m.get(key);\n    },\n    set value(item) {\n      m.set(key, item);\n    },\n  }));\n",
    tests: ["test/corpus.test.ts", "-t", "^map_entry_hashes_its_key_once.rs$"],
  },
];
