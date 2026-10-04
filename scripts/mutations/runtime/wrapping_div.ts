// Mutations of src/runtime/wrapping_div.js (ADR 0093).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "wrapping-div-min",
    breaks: "`i32::MIN.wrapping_div(-1)` is 2^31, out of range, not `MIN`",
    file: "src/runtime/wrapping_div.js",
    find: "  if (a === min && b == -1) return a;\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "integer_families"],
  },
];
