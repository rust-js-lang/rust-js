// Mutations of src/runtime/repeat.js (ADR 0093).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "repeat-none-undropped",
    breaks: "`vec![x; 0]` of a value with a destructor never drops `x`",
    file: "src/runtime/repeat.js",
    find: "    drop?.(item);\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^vec_repeat_drops.rs$"],
  },
];
