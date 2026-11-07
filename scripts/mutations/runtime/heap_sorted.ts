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
];
