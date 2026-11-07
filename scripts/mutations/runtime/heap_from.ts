// Mutations of src/runtime/heap_from.js (ADR 0093).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "heap-from-leaks-as-it-unwinds",
    breaks: "`BinaryHeap::from(v)` whose comparison panics drops none of `v`'s items, where std drops the heap it was making (#901)",
    file: "src/runtime/heap_from.js",
    find: "    drop && (() => $dropItems(heap, drop)),\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^heap_sift_unwinds.rs$"],
  },
];
