// Mutations of src/runtime/fold_of.js.
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "collect-dropping-keeps-collected",
    breaks: "a `collect` a panic stops leaves what it had collected undropped (#422)",
    file: "src/runtime/fold_of.js",
    find: "      if (drop !== undefined) $dropItems(items, drop);\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^panic_mid_iteration_drops.rs$"],
  },
];
