// Mutations of src/runtime/heap_pop.js (ADR 0093).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "heap-pop-box-ignored",
    breaks: "`$heapPop` given `boxed` gives its top as it is, so a heap's `None` is no item (#318)",
    file: "src/runtime/heap_pop.js",
    find: "  return boxed ? $some(top) : top;\n",
    replace: "  return top;\n",
    tests: ["test/corpus.test.ts", "-t", "^heap_pop_of_a_param.rs$"],
  },
];
