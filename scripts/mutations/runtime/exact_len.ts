// Mutations of crates/rust-js-program/src/runtime/exact_len.js (ADR 0093).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "exact-len-unchecked",
    breaks: "`len()` of an iterator whose `size_hint` isn't exact gives its lower bound, where std's assertion fails",
    file: "crates/rust-js-program/src/runtime/exact_len.js",
    find: "  if (upper !== lower) {",
    replace: "  if (false) {",
    tests: ["test/corpus.test.ts", "-t", "exact_len_unknown"],
  },
];
