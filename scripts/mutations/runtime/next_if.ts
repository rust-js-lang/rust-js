// Mutations of src/runtime/next_if.js (ADR 0071).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "next-if-past-end",
    breaks: "`next_if` takes what `next_back()` took",
    file: "src/runtime/next_if.js",
    find: "it.at < it.end && f(",
    replace: "it.at < it.items.length && f(",
    tests: ["test/corpus.test.ts","-t","iter_next_back"],
  },
];
