// Mutations of src/runtime/split_n.js (ADR 0093).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "splitn-drops-rest",
    breaks: "`\"a=b=c\".splitn(2, '=')` drops what's after its last piece",
    file: "src/runtime/split_n.js",
    find: "parts.slice(n - 1).join(p)",
    replace: "parts[n - 1]",
    tests: ["test/corpus.test.ts", "-t", "string_patterns"],
  },
];
