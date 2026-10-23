// Mutations of src/runtime/handle_on.js (ADR 0093).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "handle-on-unwritten",
    breaks: "a `&mut` handle on a field or an item writes nowhere",
    file: "src/runtime/handle_on.js",
    find: "      object[key] = value;\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^mut_field_given.rs$"],
  },
];
