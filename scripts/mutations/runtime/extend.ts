// Mutations of src/runtime/extend.js.
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "extend-unclosed-on-throw",
    breaks: "an `extend` whose iterator's closure panics leaves what's left of it undropped (#422)",
    file: "src/runtime/extend.js",
    find: "    it.return?.();\n    throw error;\n",
    replace: "    throw error;\n",
    tests: ["test/corpus.test.ts", "-t", "^panic_mid_iteration_drops.rs$"],
  },
];
