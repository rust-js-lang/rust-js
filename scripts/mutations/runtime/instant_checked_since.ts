// Mutations of src/runtime/instant_checked_since.js (ADR 0447).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "instant-checked-since-negative",
    breaks: "`checked_duration_since` of a later `Instant` is `Some` of a negative time, not `None`",
    file: "src/runtime/instant_checked_since.js",
    find: "a >= b ? a - b : undefined",
    replace: "a - b",
    tests: ["test/corpus.test.ts", "-t", "^clock.rs$"],
  },
];
