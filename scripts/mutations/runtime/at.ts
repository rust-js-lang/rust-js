// Mutations of src/runtime/at.js (ADR 0093).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "at-failure-unread",
    breaks: "`$at` given a `VecDeque`'s message panics with a slice's, so a deque's place written past its end does",
    file: "src/runtime/at.js",
    find: "    throw new Error(failure ?? `index out of bounds",
    replace: "    throw new Error(`index out of bounds",
    tests: ["test/corpus.test.ts", "-t", "^deque_write_out_of_bounds.rs$"],
  },
];
