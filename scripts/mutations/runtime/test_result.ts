// Mutations of src/runtime/test_result.js (ADR 0026).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "test-result-ignored",
    breaks: "a `#[test]` returning an `Err` passes",
    file: "src/runtime/test_result.js",
    find: '  if (result.TAG === "Err") {\n',
    replace: "  if (false) {\n",
    tests: ["test/browser.test.ts", "-t", "fails the way"],
  },
];
