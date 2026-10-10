// Mutations of src/runtime/peek_some.js (ADR 0071).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "peek-some-past-end",
    breaks: "a boxed `peek()` sees what `next_back()` took",
    file: "src/runtime/peek_some.js",
    find: "it.at < it.end ?",
    replace: "it.at < it.items.length ?",
    tests: ["test/corpus.test.ts","-t","iter_next_back"],
  },
];
