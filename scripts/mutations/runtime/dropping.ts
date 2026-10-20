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
  {
    name: "dispose-plain-function",
    breaks: "a `dyn Fn` of a function that carries no drop throws as it's dropped",
    file: "src/runtime/dropping.js",
    find: "  f[$disposeKey]?.();\n",
    replace: "  f[$disposeKey]();\n",
    tests: ["test/corpus.test.ts", "-t", "^dyn_fn_drops.rs$"],
  },
];
