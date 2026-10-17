// Mutations of src/runtime/closing.js (ADR 0422).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "closing-drops-none",
    breaks: "a `take` of an iterator that owns items with a destructor drops none it hasn't given as it ends",
    file: "src/runtime/closing.js",
    find: "      for (let step = it.next(); !step.done; step = it.next()) drop(step.value);\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^take_destructors.rs$"],
  },
];
