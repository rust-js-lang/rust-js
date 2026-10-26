// Mutations of src/runtime/pad.js (ADR 0919).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "pad-counts-chars-by-spread",
    breaks: "`$pad` counts its text's chars as `[...text]`, which Babel's loose spread makes one item, so React's useId pads `dalala` to `00000dalala`",
    file: "src/runtime/pad.js",
    find: "  const room = width - Array.from(text).length;\n",
    replace: "  const room = width - [...text].length;\n",
    tests: ["test/runtime-transpiled.test.ts"],
  },
];
