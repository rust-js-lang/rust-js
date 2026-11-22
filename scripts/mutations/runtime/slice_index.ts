// Mutations of crates/rust-js-program/src/runtime/slice_index.js (ADR 0418).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "slice-index-from-to-end",
    breaks: "`start..`'s dictionary ends where it starts",
    file: "crates/rust-js-program/src/runtime/slice_index.js",
    find: "  from: $boundsIndex((range) => range.start, () => undefined),\n",
    replace: "  from: $boundsIndex((range) => range.start, (range) => range.start),\n",
    tests: ["test/corpus.test.ts", "-t", "^slice_index_generic.rs$"],
  },
  {
    name: "slice-index-entry-location-dropped",
    breaks: "a `SliceIndex` dictionary's item entry drops the location it's given (#2324)",
    file: "crates/rust-js-program/src/runtime/slice_index.js",
    find: "    index: (index, items, raisedAt) => $index(items, index, raisedAt),\n",
    replace: "    index: (index, items, raisedAt) => $index(items, index),\n",
    tests: ["test/binaries.test.ts", "-t", "through a dictionary, names generic"],
  },
];
