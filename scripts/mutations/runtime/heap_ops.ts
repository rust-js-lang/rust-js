// Mutations of src/runtime/heap_ops.js (ADR 0333).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "heap-rebuild-always",
    breaks: "`append` rebuilds the whole heap, where std sifts the tail up",
    file: "src/runtime/heap_ops.js",
    find: "  if (better) {",
    replace: "  if (true) {",
    tests: ["test/corpus.test.ts", "-t", "heap_methods"],
  },
  {
    name: "heap-rebuild-never",
    breaks: "`append` sifts the tail up, where std rebuilds the whole heap",
    file: "src/runtime/heap_ops.js",
    find: "  if (better) {",
    replace: "  if (false) {",
    tests: ["test/corpus.test.ts", "-t", "heap_methods"],
  },
  {
    name: "heap-append-unswapped",
    breaks: "`append` of a larger heap puts its items after the smaller's",
    file: "src/runtime/heap_ops.js",
    find: "heap.length < other.length ? [other.slice(), heap.slice()] : [heap.slice(), other.slice()];",
    replace: "[heap.slice(), other.slice()];",
    tests: ["test/corpus.test.ts", "-t", "heap_methods"],
  },
  {
    name: "heap-retain-rebuilt-whole",
    breaks: "`retain` rebuilds from the start, not from the first item it dropped",
    file: "src/runtime/heap_ops.js",
    find: "  $heapRebuildTail(heap, Math.min(from, kept), cmp);",
    replace: "  $heapRebuildTail(heap, 0, cmp);",
    tests: ["test/corpus.test.ts", "-t", "heap_methods"],
  },
  {
    name: "peek-mut-always-sifts",
    breaks: "a `PeekMut` only read compares on its drop, calling `Ord` std's doesn't",
    file: "src/runtime/heap_ops.js",
    find: "  if (guard.changed) $siftDown(",
    replace: "  if (true) $siftDown(",
    tests: ["test/corpus.test.ts", "-t", "heap_peek_mut"],
  },
  {
    name: "peek-mut-never-sifts",
    breaks: "a changed top isn't sifted down",
    file: "src/runtime/heap_ops.js",
    find: "  if (guard.changed) $siftDown(",
    replace: "  if (false) $siftDown(",
    tests: ["test/corpus.test.ts", "-t", "heap_peek_mut"],
  },
  {
    name: "peek-mut-top-copy",
    breaks: "`*top = 0` of a number writes a copy",
    file: "src/runtime/heap_ops.js",
    find: "  if (!handle) return heap[0];",
    replace: "  return heap[0];",
    tests: ["test/corpus.test.ts", "-t", "heap_peek_mut"],
  },
];
