// Mutations of src/runtime/index_map.js (ADR 0424).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "swap-remove-shifts",
    breaks: "`swap_remove` shifts what's after the entry, not the last into its place",
    file: "src/runtime/index_map.js",
    find: "  if (at < entries.length) entries.splice(at, 0, entries.pop());\n",
    replace: "",
    tests: ["test/bound-maps.test.ts", "-t", "indexmap"],
  },
  {
    name: "index-of-by-identity",
    breaks: "where a key of a map keyed by value is, is by the key's identity, which is never there",
    file: "src/runtime/index_map.js",
    find: "  return m.keyOf === undefined ? key : m.keyOf(key);\n",
    replace: "  return key;\n",
    tests: ["test/bound-maps.test.ts", "-t", "indexmap"],
  },
];
