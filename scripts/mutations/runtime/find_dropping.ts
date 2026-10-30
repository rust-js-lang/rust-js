// Mutations of src/runtime/find_dropping.js (ADR 0093).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "find-dropping-keeps-rejected",
    breaks: "`find` of items with a destructor drops none it rejects",
    file: "src/runtime/find_dropping.js",
    find: "      drop?.(x);\n      return false;\n",
    replace: "      return false;\n",
    tests: ["test/corpus.test.ts", "-t", "^picked_items_drop.rs$"],
  },
  {
    name: "find-dropping-throw-keeps-item",
    breaks: "a `find` whose test panics, `skip_while`'s, leaves the item it was given undropped (#423)",
    file: "src/runtime/find_dropping.js",
    find: "        drop?.(x);\n        throw error;\n",
    replace: "        throw error;\n",
    tests: ["test/iterator-forms.test.ts", "-t", "^panic_part_way_drops with"],
  },
];
