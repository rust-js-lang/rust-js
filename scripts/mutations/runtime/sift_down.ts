// Mutations of src/runtime/sift_down.js (ADR 0093).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "sift-down-by-le-not-ge",
    breaks: "a heap's sift down stops at a child by `<=`, where std's asks `>=`, and an impl's own `ge` isn't asked (#937)",
    file: "src/runtime/sift_down.js",
    find: "      if (ord.ge(item, heap[child])) {\n",
    replace: "      if (!ord.lt(item, heap[child])) {\n",
    tests: ["test/corpus.test.ts", "-t", "^generic_compare_by_impls.rs$"],
  },
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
