// Mutations of src/runtime/sign_negative.js (ADR 0093).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "sign-negative-zero",
    breaks: "`(-0.0).is_sign_negative()` is false",
    file: "src/runtime/sign_negative.js",
    find: "  return x < 0 || Object.is(x, -0);",
    replace: "  return x < 0;",
    tests: ["test/corpus.test.ts", "-t", "number_methods"],
  },
];
