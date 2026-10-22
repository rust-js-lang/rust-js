// Mutations of src/runtime/checked_index.js (ADR 0093).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "index-panic-message",
    breaks: "an index out of bounds panics with another message than Rust's",
    file: "src/runtime/checked_index.js",
    find: "    throw new Error(failure ?? `index out of bounds: the len is",
    replace: "    throw new Error(failure ?? `index out of bounds: the length is",
    tests: ["test/corpus.test.ts", "-t", "index_out_of_bounds"],
  },
  {
    name: "index-failure-unread",
    breaks: "`$index` given a `VecDeque`'s message panics with a slice's",
    file: "src/runtime/checked_index.js",
    find: "    throw new Error(failure ?? `index out of bounds",
    replace: "    throw new Error(`index out of bounds",
    tests: ["test/corpus.test.ts", "-t", "^deque_index_out_of_bounds.rs$"],
  },
];
