// Mutations of src/runtime/nth.js (ADR 0093).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "nth-own-some-unboxed",
    breaks: "$nthSome boxes its Some",
    file: "src/runtime/nth.js",
    find: "  const step = it.nth(k);\n  return step.done ? undefined : $some(step.value);",
    replace: "  const step = it.nth(k);\n  return step.done ? undefined : step.value;",
    tests: ["test/corpus.test.ts", "-t", "^own_nth_through_adapters.rs$"],
  },
  {
    name: "last-of-js-iterator-asked",
    breaks: "generic code's `last()` of JS's own iterator, which has no `last`, throws `it.last is not a function` (#496)",
    file: "src/runtime/nth.js",
    find: "  if (typeof it.last === \"function\") return it.last();\n",
    replace: "  return it.last();\n",
    tests: ["test/corpus.test.ts", "-t", "^generic_own_last.rs$"],
  },
];
