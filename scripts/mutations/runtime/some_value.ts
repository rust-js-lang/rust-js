// Mutations of src/runtime/some_value.js (ADR 0051).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "some-value-null-forgotten",
    breaks: "what's in a box of JS's `null` is `undefined`",
    file: "src/runtime/some_value.js",
    find: "x.$someNone === 0 ? x.$value :",
    replace: "x.$someNone === 0 ? undefined :",
    tests: ["test/bindings.test.ts","-t","keeps the null"],
  },
  {
    name: "some-value-box-null-forgotten",
    breaks: "a box of a box of JS's `null` gives one of `undefined`",
    file: "src/runtime/some_value.js",
    find: "{ ...x, $someNone: x.$someNone - 1 }",
    replace: "{ $someNone: x.$someNone - 1 }",
    tests: ["test/bindings.test.ts","-t","keeps the null"],
  },
];
