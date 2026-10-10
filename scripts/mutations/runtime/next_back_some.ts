// Mutations of src/runtime/next_back_some.js (ADR 0071).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "next-back-some-unboxed",
    breaks: "`next_back()` of `Option`s gives `None` for `Some(None)`",
    file: "src/runtime/next_back_some.js",
    find: "$some(it.items[--it.end])",
    replace: "it.items[--it.end]",
    tests: ["test/corpus.test.ts","-t","iter_next_back"],
  },
];
