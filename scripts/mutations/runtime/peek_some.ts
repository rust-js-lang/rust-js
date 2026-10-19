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
  {
    name: "peek-some-lazy-steps-again",
    breaks: "`peek()` of a lazy peekable, as an `Option` of a reference, steps its source each time",
    file: "src/runtime/peek_some.js",
    find: "    const step = it.peek();\n    return step.done ? undefined : $some(step.value);\n",
    replace: "    const step = it.next();\n    return step.done ? undefined : $some(step.value);\n",
    tests: ["test/corpus.test.ts", "-t", "^peekable_lazy.rs$"],
  },
];
