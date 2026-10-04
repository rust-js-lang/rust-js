// Mutations of src/runtime/split_terminator.js (ADR 0093).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "split-terminator-keeps-last",
    breaks: "`\"a,b,\".split_terminator(',')` keeps its empty last piece",
    file: "src/runtime/split_terminator.js",
    find: "  if (parts.at(-1) === \"\") parts.pop();\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "string_patterns"],
  },
];
