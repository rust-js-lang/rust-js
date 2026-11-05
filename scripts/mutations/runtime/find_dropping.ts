// Mutations of src/runtime/find_dropping.js (ADR 0093).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "find-dropping-keeps-rejected",
    breaks: "`find` of items with a destructor drops none it rejects",
    file: "src/runtime/find_dropping.js",
    find: "$kept(test, drop, x)",
    replace: "$kept(test, undefined, x)",
    tests: ["test/corpus.test.ts", "-t", "^picked_items_drop.rs$"],
  },
  {
    name: "find-dropping-throw-keeps-item",
    breaks: "a `find` whose test panics, `skip_while`'s, leaves the item it was given undropped, asked past `$kept` (#423, #738)",
    file: "src/runtime/find_dropping.js",
    find: "$kept(test, drop, x)",
    replace: "(test(x) || (drop?.(x), false))",
    tests: ["test/iterator-forms.test.ts", "-t", "^panic_part_way_drops with"],
  },
];
