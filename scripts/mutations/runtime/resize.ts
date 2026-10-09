// Mutations of src/runtime/resize.js (ADR 0093).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "resize-moves-no-original",
    breaks: "the value given isn't the last item, cloned like the rest",
    file: "src/runtime/resize.js",
    find: "  v.push(item);\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "vec_edits"],
  },
];
