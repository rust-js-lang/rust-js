// Mutations of src/runtime/closing.js (ADR 0422).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "closing-drops-none",
    breaks: "a `take` of an iterator that owns items with a destructor drops none it hasn't given as it ends",
    file: "src/runtime/closing.js",
    find: "        drop?.(it);\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^take_destructors.rs$"],
  },
  {
    name: "closing-twice",
    breaks: "an iterator closed twice, by `chain`'s `yield*` and its end, is dropped twice",
    file: "src/runtime/closing.js",
    find: "      if (!closed) {\n",
    replace: "      if (true) {\n",
    tests: ["test/corpus.test.ts", "-t", "^adapter_drops.rs$"],
  },
];
