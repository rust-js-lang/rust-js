// Mutations of crates/rust-js-program/src/runtime/index_map.js (ADR 0424).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "swap-remove-shifts",
    breaks: "`swap_remove` shifts what's after the entry, not the last into its place",
    file: "crates/rust-js-program/src/runtime/index_map.js",
    find: "  if (at < entries.length) entries.splice(at, 0, entries.pop());\n",
    replace: "",
    tests: ["test/bound-maps.test.ts", "-t", "indexmap"],
  },
  {
    name: "index-of-by-identity",
    breaks: "where a key of a map keyed by value is, is by the key's identity, which is never there",
    file: "crates/rust-js-program/src/runtime/index_map.js",
    find: "  return m.keyOf === undefined ? key : m.keyOf(key);\n",
    replace: "  return key;\n",
    tests: ["test/bound-maps.test.ts", "-t", "indexmap"],
  },
  {
    name: "place-index-unchecked",
    breaks: "`m[i]` of an `IndexMap` past its end is `undefined`, not indexmap's panic (#2437)",
    file: "crates/rust-js-program/src/runtime/index_map.js",
    find: "  if (i >= m.size) throw $stringPanic(`index out of bounds: the len is ${m.size} but the index is ${i}`, raisedAt);\n",
    replace: "",
    tests: ["test/bound-maps.test.ts", "-t", "indexmap"],
  },
  {
    name: "set-place-index-read-as-a-maps",
    breaks: "`s[i]` of an `IndexSet` reads its item as a map's entry, its second character, not the item (#2437)",
    file: "crates/rust-js-program/src/runtime/index_map.js",
    find: "    if (at++ === i) return m instanceof Map ? entry[1] : entry;\n",
    replace: "    if (at++ === i) return entry[1];\n",
    tests: ["test/bound-maps.test.ts", "-t", "indexmap"],
  },
];
