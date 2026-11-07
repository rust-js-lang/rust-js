// Mutations of src/runtime/sift_down.js (ADR 0093).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "sift-down-hole-unwritten",
    breaks: "a heap's sift down whose comparison panics leaves its hole holding the child moved over it: one item dropped twice, the moving one never (#901)",
    file: "src/runtime/sift_down.js",
    find: "    return pos;\n  } finally {\n    heap[pos] = item;\n  }\n}\n\n",
    replace: "    heap[pos] = item;\n    return pos;\n  } finally {\n  }\n}\n\n",
    tests: ["test/corpus.test.ts", "-t", "^heap_sift_unwinds.rs$"],
  },
  {
    name: "sift-to-bottom-hole-unwritten",
    breaks: "`pop`'s sift to the bottom whose comparison panics leaves its hole holding the child moved over it (#901)",
    file: "src/runtime/sift_down.js",
    find: "  } finally {\n    heap[pos] = item;\n  }\n  $siftUp(",
    replace: "    heap[pos] = item;\n  } finally {\n  }\n  $siftUp(",
    tests: ["test/corpus.test.ts", "-t", "^heap_sift_unwinds.rs$"],
  },
];
