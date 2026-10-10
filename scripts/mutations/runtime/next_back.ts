// Mutations of src/runtime/next_back.js (ADR 0071).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "next-back-from-front",
    breaks: "`next_back()` takes the first item",
    file: "src/runtime/next_back.js",
    find: "it.items[--it.end]",
    replace: "it.items[it.at++]",
    tests: ["test/corpus.test.ts","-t","iter_next_back"],
  },
  {
    name: "next-back-past-front",
    breaks: "`next_back()` takes items `next()` took",
    file: "src/runtime/next_back.js",
    find: "it.at < it.end ?",
    replace: "it.end > 0 ?",
    tests: ["test/corpus.test.ts","-t","iter_next_back"],
  },
];
