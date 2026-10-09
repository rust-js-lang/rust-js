// Mutations of src/runtime/once_set.js (ADR 0317).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "once-set-overwrites",
    breaks: "a second `set` replaces what the first set, and is `Ok`",
    file: "src/runtime/once_set.js",
    find: '  if (cell.value !== undefined) return { TAG: "Err", _0: value };\n',
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "once_cells"],
  },
  {
    name: "once-set-unboxed",
    breaks: "a `()` or `None` set looks unset",
    file: "src/runtime/once_set.js",
    find: "  cell.value = $some(value);\n",
    replace: "  cell.value = value;\n",
    tests: ["test/corpus.test.ts", "-t", "once_cells"],
  },
];
