// Mutations of src/runtime/cycle.js (ADR 0181).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "cycle-of-none-gives-one",
    breaks: "`cycle()` of an iterator with nothing left gives an item, where Rust's gives none",
    file: "src/runtime/cycle.js",
    find: "  if (items.length === 0) return;\n",
    replace: "  if (items.length === 0) yield undefined;\n",
    tests: ["test/corpus.test.ts", "-t", "^iter_cycle.rs$"],
  },
  {
    name: "cycle-restart-drop-first",
    breaks: "`Cycle` drops the iterator it replaces before it clones the next, not after",
    file: "src/runtime/cycle.js",
    find: "        const fresh = clone(orig);\n        drop?.(it);\n",
    replace: "        drop?.(it);\n        const fresh = clone(orig);\n",
    tests: ["test/corpus.test.ts", "-t", "^cycled_iterators.rs$"],
  },
  {
    name: "cycle-close-order",
    breaks: "a closed `Cycle` drops what it steps before the clone it keeps, not after",
    file: "src/runtime/cycle.js",
    find: "        drop?.(orig);\n        drop?.(it);\n",
    replace: "        drop?.(it);\n        drop?.(orig);\n",
    tests: ["test/corpus.test.ts", "-t", "^cycled_iterators.rs$"],
  },
];
