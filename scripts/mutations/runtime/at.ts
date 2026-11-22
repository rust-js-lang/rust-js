// Mutations of crates/rust-js-program/src/runtime/at.js (ADR 0093).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "at-failure-unread",
    breaks: "`$at` given a `VecDeque`'s message panics with a slice's, so a deque's place written past its end does",
    file: "crates/rust-js-program/src/runtime/at.js",
    find: "    throw $stringPanic(failure ?? `index out of bounds",
    replace: "    throw $stringPanic(`index out of bounds",
    tests: ["test/corpus.test.ts", "-t", "^deque_write_out_of_bounds.rs$"],
  },
];
