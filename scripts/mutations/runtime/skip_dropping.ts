// Mutations of src/runtime/skip_dropping.js (ADR 0422).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "skip-dropping-keeps-skipped",
    breaks: "`skip` of items with a destructor drops none it skips",
    file: "src/runtime/skip_dropping.js",
    find: "        drop(step.value);\n      }\n      return it.next();\n",
    replace: "      }\n      return it.next();\n",
    tests: ["test/corpus.test.ts", "-t", "^take_destructors.rs$"],
  },
];
