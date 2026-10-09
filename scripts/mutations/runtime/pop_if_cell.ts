// Mutations of src/runtime/pop_if_cell.js (ADR 0093).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "pop-if-unwritten",
    breaks: "what the closure changes of the last item isn't kept",
    file: "src/runtime/pop_if_cell.js",
    find: "  v[v.length - 1] = last.value;\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "vec_edits"],
  },
];
