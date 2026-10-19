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
  {
    name: "next-if-lazy-keeps-peeked",
    breaks: "`next_if` of a lazy peekable that takes the item leaves it to be given again",
    file: "src/runtime/next_if.js",
    find: "    return !step.done && f(step.value) ? it.next().value : undefined;\n",
    replace: "    return !step.done && f(step.value) ? step.value : undefined;\n",
    tests: ["test/corpus.test.ts", "-t", "^peekable_lazy.rs$"],
  },
];
