// Mutations of test/oracle.ts (ADR 0093).
import type { Mutation } from "../mutations";

export const mutations: Mutation[] = [
  {
    name: "outcomes-compared-loosely",
    breaks: "two outcomes are compared loosely, so -0 passes for 0 and 1n for 1 (ADR 0088)",
    file: "test/oracle.ts",
    find: "  return Bun.deepEquals(a, b, true);\n",
    replace: "  return Bun.deepEquals(a, b, false);\n",
    tests: ["test/oracle.test.ts", "-t", "values are compared strictly"],
  },
];
