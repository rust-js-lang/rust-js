// Mutations of src/runtime/join_with.js (ADR 0322).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "join-with-separator-whole",
    breaks: "a slice separator goes between the lists as one item, not its items",
    file: "src/runtime/join_with.js",
    find: "      for (const item of spread ? sep : [sep]) out.push(copy(item));\n",
    replace: "      for (const item of [sep]) out.push(copy(item));\n",
    tests: ["test/corpus.test.ts", "-t", "text_aliases"],
  },
];
