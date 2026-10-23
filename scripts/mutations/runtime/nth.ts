// Mutations of src/runtime/nth.js (ADR 0093).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "nth-own-some-unboxed",
    breaks: "$nthSome boxes its Some",
    file: "src/runtime/nth.js",
    find: "  return step.done ? undefined : $some(step.value);",
    replace: "  return step.done ? undefined : step.value;",
    tests: ["test/corpus.test.ts", "-t", "^own_nth_through_adapters.rs$"],
  },
];
