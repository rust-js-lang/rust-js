// Mutations of src/runtime/sift_up.js (ADR 0093).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "sift-up-hole-unwritten",
    breaks: "a heap's sift up whose comparison panics leaves its hole holding the parent moved over it: one item dropped twice, the pushed one never (#901)",
    file: "src/runtime/sift_up.js",
    find: "    return pos;\n  } finally {\n    heap[pos] = item;\n  }\n",
    replace: "    heap[pos] = item;\n    return pos;\n  } finally {\n  }\n",
    tests: ["test/corpus.test.ts", "-t", "^heap_sift_unwinds.rs$"],
  },
];
