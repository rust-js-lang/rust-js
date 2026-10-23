// Mutations of src/runtime/settled.js (#172).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "settled-unboxed",
    breaks: "a thenable a future gives is resolved as it is, so the promise adopts it",
    file: "src/runtime/settled.js",
    find: "  return thenable ? { $settled: value } : value;",
    replace: "  return value;",
    tests: ["test/lowering-functions.test.ts", "-t", "thenable"],
  },
];
