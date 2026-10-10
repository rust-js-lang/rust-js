// Mutations of src/runtime/rest_str.js (ADR 0071).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "rest-str-past-end",
    breaks: "`as_str()` has what `next_back()` took",
    file: "src/runtime/rest_str.js",
    find: "it.items.slice(it.at, it.end)",
    replace: "it.items.slice(it.at)",
    tests: ["test/corpus.test.ts","-t","iter_next_back"],
  },
];
