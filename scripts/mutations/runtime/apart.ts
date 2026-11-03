// Mutations of src/runtime/apart.js (ADR 0093).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "apart-shares",
    breaks: "`$apart` gives the object itself, so what generic code moves out of its box is changed by the write-back",
    file: "src/runtime/apart.js",
    find: "  if (Array.isArray(value)) return value.slice();",
    replace: "  if (Array.isArray(value)) return value;",
    tests: ["test/corpus.test.ts", "-t", "^generic_mut_moved_out.rs$"],
  },
  {
    name: "apart-std-struct-shares-steps",
    breaks:
      "`$apart` of std's compiled struct copies its methods, which step the struct itself: what `mem::replace` moves out steps what's written there (#537)",
    file: "src/runtime/apart.js",
    find: "  if (value?.$copy !== undefined) return value.$copy();\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^iterator_values_meet.rs$"],
  },
];
