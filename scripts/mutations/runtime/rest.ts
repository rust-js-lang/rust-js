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
  {
    name: "rest-of-js-iterator",
    breaks: "what's left of a generic iterator stepped by hand is read as a stepped array's",
    file: "src/runtime/rest.js",
    find: "  if (it.items === undefined) {\n    return Array.from(it);\n  }\n",
    replace: "",
    tests: ["test/corpus.test.ts","-t","^iterator_stepped_by_hand.rs$"],
  },
];
