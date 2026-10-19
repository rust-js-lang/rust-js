// Mutations of src/runtime/dropping.js (ADR 0484).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "dropping-carries-nothing",
    breaks: "`$dropping` gives back the closure carrying no drop",
    file: "src/runtime/dropping.js",
    find: "  f[$disposeKey] = drop;\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^returned_closure_drops.rs$"],
  },
];
