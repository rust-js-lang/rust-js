// Mutations of src/runtime/heap_sorted.js (ADR 0093).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "heap-sorted-leaks-as-it-unwinds",
    breaks: "`into_sorted_vec` whose comparison panics drops none of the heap's items, where std drops its `self` (#901)",
    file: "src/runtime/heap_sorted.js",
    find: "    drop && (() => $dropItems(heap, drop)),\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^heap_sift_unwinds.rs$"],
  },
  {
    name: "heap-sorted-capacity-its-length",
    breaks: "`into_sorted_vec()` gives a `Vec` whose capacity is its length, not the heap's, whose buffer std sorts in place: 3 where native Rust's is 12 (#1531)",
    file: "src/runtime/heap_sorted.js",
    find: "  return $inPlaceCapacity(heap, items, 1, 1);\n",
    replace: "  return heap;\n",
    tests: ["test/corpus.test.ts", "-t", "^binary_heap_keeps_its_vecs_capacity.rs$"],
  },
];
