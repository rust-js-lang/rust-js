// Mutations of src/runtime/chunks.js (ADR 0335).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "chunks-past-end",
    breaks: "the last chunk runs past the end, which a view refuses",
    file: "src/runtime/chunks.js",
    find: "cut(v, i * size, Math.min(i * size + size, v.length))",
    replace: "cut(v, i * size, i * size + size)",
    tests: ["test/corpus.test.ts","-t","slice_views"],
  },
];
