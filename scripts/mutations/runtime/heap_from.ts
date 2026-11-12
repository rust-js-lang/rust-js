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
  {
    name: "heap-from-capacity-its-length",
    breaks: "a `BinaryHeap` made of a `Vec`, `from(v)` or a collect, has a capacity of its length, not the `Vec`'s whose buffer it keeps: 34 where native Rust's is 64 (#1531)",
    file: "src/runtime/heap_from.js",
    find: "  return $inPlaceCapacity(heap, items, 1, 1);\n",
    replace: "  return heap;\n",
    tests: ["test/corpus.test.ts", "-t", "^binary_heap_keeps_its_vecs_capacity.rs$"],
  },
];
