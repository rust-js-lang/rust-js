// Mutations of src/runtime/peek.js (ADR 0071).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "peek-past-end",
    breaks: "`peek()` sees what `next_back()` took",
    file: "src/runtime/peek.js",
    find: "  return it.at < it.end ? it.items[it.at] : undefined;",
    replace: "  return it.items[it.at];",
    tests: ["test/corpus.test.ts","-t","iter_next_back"],
  },
];
