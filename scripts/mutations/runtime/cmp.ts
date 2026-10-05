// Mutations of src/runtime/cmp.js.
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "cmp-by-utf16-units",
    breaks: "strings compare by UTF-16 units, 🦀 before a full-width `！`, where Rust's order is by code point",
    file: "src/runtime/cmp.js",
    find: "      if (x >= 0xd800 && y >= 0xd800 && x < 0xe000 !== y < 0xe000) return x < 0xe000 ? 1 : -1;\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "code_point_order"],
  },
];
