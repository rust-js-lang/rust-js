// Mutations of src/runtime/taking.js (ADR 0542).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "taking-back-uncounted",
    breaks: "an explicit `take` stepped from its back doesn't count down, so it gives more than it takes",
    file: "src/runtime/taking.js",
    find: "    const past = Math.max(taking.iter.len() - taking.n, 0);\n    taking.n--;\n",
    replace: "    const past = Math.max(taking.iter.len() - taking.n, 0);\n",
    tests: ["test/corpus.test.ts", "-t", "^cloned_counted_adapters.rs$"],
  },
  {
    name: "taking-length-its-source",
    breaks: "an explicit `take`'s length is its source's, not the fewer of that and how many it takes",
    file: "src/runtime/taking.js",
    find: "  taking.len = () => Math.min(taking.iter.len(), taking.n);\n",
    replace: "  taking.len = () => taking.iter.len();\n",
    tests: ["test/corpus.test.ts", "-t", "^cloned_counted_adapters.rs$"],
  },
  {
    name: "taking-front-uncounted",
    breaks: "an explicit `take` stepped from its front never stops",
    file: "src/runtime/taking.js",
    find: "      taking.n--;\n      return taking.iter.next();\n",
    replace: "      return taking.iter.next();\n",
    tests: ["test/corpus.test.ts", "-t", "^cloned_counted_adapters.rs$"],
  },
];
