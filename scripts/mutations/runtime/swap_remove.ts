// Mutations of crates/rust-js-program/src/runtime/swap_remove.js (ADR 0093).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "swap-remove-keeps-last",
    breaks: "the last item stays, duplicated where the removed one was",
    file: "crates/rust-js-program/src/runtime/swap_remove.js",
    find: "  v.pop();\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "vec_edits"],
  },
];
