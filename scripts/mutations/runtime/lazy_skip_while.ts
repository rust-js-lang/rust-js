// Mutations of src/runtime/lazy_skip_while.js (ADR 0093).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "skip-while-try-fold-by-next",
    breaks: "skip_while's tryFold is its first step, then its source's",
    file: "src/runtime/lazy_skip_while.js",
    find: "    return $lent(source).tryFold(folded, f, broke);",
    replace: "    for (let step = source.next(); !step.done; step = source.next()) {\n      folded = f(folded, step.value);\n      if (broke(folded)) return folded;\n    }\n    return folded;",
    tests: ["test/corpus.test.ts", "-t", "skip_try_fold"],
  },
  {
    name: "skip-while-next-by-next",
    breaks: "skip_while's next is its source's find",
    file: "src/runtime/lazy_skip_while.js",
    find: "      const found = $lent(source).tryFold(",
    replace: "      const found = new $Steps(() => source.next()).tryFold(",
    tests: ["test/corpus.test.ts", "-t", "skip_try_fold"],
  },
];
