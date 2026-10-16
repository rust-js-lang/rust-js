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
];
