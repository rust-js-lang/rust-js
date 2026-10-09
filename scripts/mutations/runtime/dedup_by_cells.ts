// Mutations of src/runtime/dedup_by_cells.js (ADR 0093).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "dedup-cells-unwritten",
    breaks: "what the closure changes of the kept item isn't kept",
    file: "src/runtime/dedup_by_cells.js",
    find: "    v[n - 1] = b.value;\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "vec_edits"],
  },
];
