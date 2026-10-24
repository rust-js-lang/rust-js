// Mutations of src/runtime/iter_cmp.js (ADR 0093).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "iter-cmp-shorter-greater",
    breaks: "an iterator that ends before another compares `Greater`, where it's `Less`",
    file: "src/runtime/iter_cmp.js",
    find: "  return y.next().done ? 0 : -1;\n",
    replace: "  return y.next().done ? 0 : 1;\n",
    tests: ["test/corpus.test.ts", "-t", "^iter_compare.rs$"],
  },
  {
    name: "iter-cmp-by-next",
    breaks: "cmp steps its left side by tryFold",
    file: "src/runtime/iter_cmp.js",
    find: "  const stopped = $lent(x).tryFold(",
    replace: "  const stopped = new $Steps(() => x.next()).tryFold(",
    tests: ["test/corpus.test.ts", "-t", "skip_try_fold"],
  },
];
