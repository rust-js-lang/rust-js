// Mutations of src/runtime/repeating.js (ADR 0422).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "repeating-closed-undropped",
    breaks: "a `repeat`'s value isn't dropped when its iterator is closed",
    file: "src/runtime/repeating.js",
    find: "      drop?.(value);\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^repeat_drops.rs$"],
  },
];
