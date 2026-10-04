// Mutations of src/runtime/assign.js (ADR 0093).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "assign-keeps-old-fields",
    breaks: "an enum replaced by another variant in place keeps the old one's fields, so `==` of it is false",
    file: "src/runtime/assign.js",
    find: "    for (const key of Object.keys(target)) if (!(key in value)) delete target[key];\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "replace_through_mut"],
  },
];
