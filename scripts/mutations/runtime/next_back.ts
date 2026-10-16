// Mutations of src/runtime/next_back.js (ADRs 0071, 0429).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "next-back-steps-front",
    breaks: "`next_back()` takes the first item",
    file: "src/runtime/next_back.js",
    find: "  const step = it.back();\n",
    replace: "  const step = it.next();\n",
    tests: ["test/corpus.test.ts","-t","iter_next_back"],
  },
];
