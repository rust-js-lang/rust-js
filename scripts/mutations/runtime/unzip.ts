// Mutations of src/runtime/unzip.js (ADR 0093).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "unzip-swapped",
    breaks: "`unzip()` gives the second parts first",
    file: "src/runtime/unzip.js",
    find: "    a.push(x);\n    b.push(y);\n",
    replace: "    a.push(y);\n    b.push(x);\n",
    tests: ["test/corpus.test.ts","-t","iterator_sources"],
  },
  {
    name: "unzip-steps",
    breaks: "`unzip` of the crate's iterator steps it by `next`, not by its own `fold`",
    file: "src/runtime/unzip.js",
    find: "  Iterator.from(pairs).forEach(([x, y]) => {\n",
    replace: "  Iterator.prototype.forEach.call(Iterator.from(pairs), ([x, y]) => {\n",
    tests: ["test/corpus.test.ts",  "-t",  "^own_fold_and_nth.rs$"],
  },
];
