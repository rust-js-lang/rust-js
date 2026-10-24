// Mutations of src/runtime/cmp_items.js (ADR 0465).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "iter-eq-pair-undropped",
    breaks: "`eq` of items with a destructor drops none of the other side's pairs",
    file: "src/runtime/cmp_items.js",
    find: "      dropB?.(q.value);\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^adapter_drops.rs$"],
  },
  {
    name: "iter-eq-unclosed",
    breaks: "`eq` doesn't close what it compared once it ends, so what's left of either isn't dropped",
    file: "src/runtime/cmp_items.js",
    find: "    y.return?.();\n    x.return?.();\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^adapter_drops.rs$"],
  },
  {
    name: "iter-eq-by-next",
    breaks: "eq steps its left side by tryFold",
    file: "src/runtime/cmp_items.js",
    find: "    const stopped = $lent(x).tryFold(",
    replace: "    const stopped = new $Steps(() => x.next()).tryFold(",
    tests: ["test/corpus.test.ts", "-t", "skip_try_fold"],
  },
];
