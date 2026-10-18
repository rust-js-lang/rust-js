// Mutations of src/runtime/instant_since.js (ADR 0447).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "instant-since-negative",
    breaks: "the time since a later `Instant` is negative, not zero as std saturates",
    file: "src/runtime/instant_since.js",
    find: "a > b ? a - b : 0n",
    replace: "a - b",
    tests: ["test/corpus.test.ts", "-t", "^clock.rs$"],
  },
];
