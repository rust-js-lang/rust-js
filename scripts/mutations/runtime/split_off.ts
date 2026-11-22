// Mutations of crates/rust-js-program/src/runtime/split_off.js (ADR 1184).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "deque-split-off-vec-message",
    breaks: "a `VecDeque`'s `split_off` past its end panics with a `Vec`'s message, not its own `` `at` out of bounds ``",
    file: "crates/rust-js-program/src/runtime/split_off.js",
    find: "throw deque ? $panic(\"`at` out of bounds\", raisedAt) : $stringPanic",
    replace: "throw $stringPanic",
    tests: ["test/binaries.test.ts", "-t", "std raises"],
  },
];
