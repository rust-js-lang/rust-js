// Mutations of src/runtime/find_by.js (ADR 0093).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "find-by-last",
    breaks: "`find` by a closure gives the last match, as `rfind` does",
    file: "src/runtime/find_by.js",
    find: "      if (!last) return found;\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "text_predicates"],
  },
];
