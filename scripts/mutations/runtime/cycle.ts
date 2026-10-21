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
    find: "        const fresh = clone(cycled.orig);\n        drop?.(cycled.iter);\n",
    replace: "        drop?.(cycled.iter);\n        const fresh = clone(cycled.orig);\n",
    tests: ["test/corpus.test.ts", "-t", "^cycled_iterators.rs$"],
  },
  {
    name: "cycle-close-order",
    breaks: "a closed `Cycle` drops what it steps before the clone it keeps, not after",
    file: "src/runtime/cycle.js",
    find: "        drop?.(cycled.orig);\n        drop?.(cycled.iter);\n",
    replace: "        drop?.(cycled.iter);\n        drop?.(cycled.orig);\n",
    tests: ["test/corpus.test.ts", "-t", "^cycled_iterators.rs$"],
  },
  {
    name: "cycle-clone-restarts",
    breaks: "a held `Cycle`'s clone steps from what it started with, not where it is",
    file: "src/runtime/cycle.js",
    find: "  cycled[$cloneKey] = () => $cycleFrom(clone(cycled.orig), clone(cycled.iter), clone, next, boxed, drop);\n",
    replace: "  cycled[$cloneKey] = () => $cycleFrom(clone(cycled.orig), clone(cycled.orig), clone, next, boxed, drop);\n",
    tests: ["test/corpus.test.ts", "-t", "^held_cycle.rs$"],
  },
  {
    name: "cycle-hint-of-empty-unknown",
    breaks: "an empty `Cycle`'s size hint isn't `(0, Some(0))`",
    file: "src/runtime/cycle.js",
    find: "    if (low === 0 && high === 0) return [0, 0];\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^held_cycle.rs$"],
  },
];
