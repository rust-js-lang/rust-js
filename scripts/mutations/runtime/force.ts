// Mutations of src/runtime/force.js (ADR 0318).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "force-every-time",
    breaks: "`init` runs on every use, not only the first",
    file: "src/runtime/force.js",
    find: "    lazy.init = undefined;\n",
    replace: "    lazy.init = init;\n",
    tests: ["test/corpus.test.ts", "-t", "lazy_cells"],
  },
  {
    name: "force-unpoisoned",
    breaks: "an init that reads its cell recurses, not std's panic",
    file: "src/runtime/force.js",
    find: "    lazy.init = null;\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "lazy_cell_reentrant"],
  },
];
