// Mutations of src/runtime/mut_at.js (ADR 0093).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "mut-at-no-end",
    breaks: "`last_mut()` of numbers is `None`: a negative index isn't counted from the end",
    file: "src/runtime/mut_at.js",
    find: "  const at = i < 0 ? v.length + i : i;\n",
    replace: "  const at = i;\n",
    tests: ["test/corpus.test.ts", "-t", "std_mut_items"],
  },
];
