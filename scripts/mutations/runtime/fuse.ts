// Mutations of src/runtime/fuse.js (ADR 0473).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "fuse-keeps-every-source",
    breaks: "a `Fuse` of what isn't a `FusedIterator` keeps it as it ends, dropping it only with the `Fuse`",
    file: "src/runtime/fuse.js",
    find: "      if (step.done && !fused) {\n",
    replace: "      if (step.done && false) {\n",
    tests: ["test/corpus.test.ts", "-t", "^fused_iterators.rs$"],
  },
];
