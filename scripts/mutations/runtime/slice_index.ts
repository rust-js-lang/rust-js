// Mutations of src/runtime/slice_index.js (ADR 0418).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "slice-index-from-to-end",
    breaks: "`start..`'s dictionary ends where it starts",
    file: "src/runtime/slice_index.js",
    find: "  from: $boundsIndex((range) => range.start, () => undefined),\n",
    replace: "  from: $boundsIndex((range) => range.start, (range) => range.start),\n",
    tests: ["test/corpus.test.ts", "-t", "^slice_index_generic.rs$"],
  },
];
