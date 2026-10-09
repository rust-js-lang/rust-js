// Mutations of src/runtime/splice.js (ADR 0093).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "splice-replaces-nothing",
    breaks: "`splice` removes the range but puts nothing in its place",
    file: "src/runtime/splice.js",
    find: "end - start, ...replacement",
    replace: "end - start",
    tests: ["test/corpus.test.ts", "-t", "vec_edits"],
  },
];
