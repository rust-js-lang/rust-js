// Mutations of src/runtime/rest.js (ADR 0071).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "rest-past-end",
    breaks: "what's left of a stepped iterator has what `next_back()` took",
    file: "src/runtime/rest.js",
    find: "it.items.slice(it.at, it.end)",
    replace: "it.items.slice(it.at)",
    tests: ["test/corpus.test.ts","-t","iter_next_back"],
  },
];
