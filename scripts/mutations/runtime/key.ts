// Mutations of src/runtime/key.js (ADR 0093).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "key-of-none-fields",
    breaks: "a struct with a `None` left out and one with it there are two keys of a set",
    file: "src/runtime/key.js",
    find: "      const keys = Object.keys(value).filter((k) => value[k] != null);\n",
    replace: "      const keys = Object.keys(value);\n",
    tests: ["test/lowering.test.ts", "-t", "literal None is left out"],
  },
];
