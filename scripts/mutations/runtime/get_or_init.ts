// Mutations of src/runtime/get_or_init.js (ADR 0317).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "get-or-init-reentrant",
    breaks: "an init that sets the cell itself is overwritten, not std's panic",
    file: "src/runtime/get_or_init.js",
    find: '    if (cell.value !== undefined) throw new Error("reentrant init");\n',
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "once_cell_reentrant"],
  },
  {
    name: "get-or-init-every-time",
    breaks: "`f` runs on every call, not only the first",
    file: "src/runtime/get_or_init.js",
    find: "  if (cell.value === undefined) {\n",
    replace: "  if (true) {\n",
    tests: ["test/corpus.test.ts", "-t", "once_cells"],
  },
  {
    name: "get-or-init-boxed",
    breaks: "a `None` it made is given as `Some`'s box",
    file: "src/runtime/get_or_init.js",
    find: "  return $someValue(cell.value);\n",
    replace: "  return cell.value;\n",
    tests: ["test/corpus.test.ts", "-t", "once_cells"],
  },
];
