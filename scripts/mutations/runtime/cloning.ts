// Mutations of src/runtime/cloning.js (ADR 0522).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "cloning-carries-nothing",
    breaks: "`$cloning` gives back the closure carrying no clone",
    file: "src/runtime/cloning.js",
    find: "  f[$cloneKey] = clone;\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^closure_clone.rs$"],
  },
];
