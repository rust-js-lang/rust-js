// Mutations of src/runtime/next_if.js (ADR 0071).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "next-if-lazy-keeps-peeked",
    breaks: "`next_if` of a lazy peekable that takes the item leaves it to be given again",
    file: "src/runtime/next_if.js",
    find: "  if (item !== undefined && f(item)) return item;\n",
    replace: "  if (item !== undefined && f(item)) {\n    peekable.peeked = $some(item);\n    return item;\n  }\n",
    tests: ["test/corpus.test.ts", "-t", "^peekable_lazy.rs$"],
  },
];
