// Mutations of src/runtime/char_step.js (ADR 0364).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "char-step-into-surrogates",
    breaks: "a `char` range steps into the surrogates, which no `char` is",
    file: "src/runtime/char_step.js",
    find: "  if (code >= 0xd800 && code <= 0xdfff) code = by > 0 ? 0xe000 : 0xd7ff;\n",
    replace: "",
    tests: ["test/mir.test.ts","-t","char_ranges_stepped"],
  },
];
