// Mutations of src/runtime/collect_options.js (ADR 0093).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    "name": "collect-options-not-into",
    "breaks": "`collect()` into an `Option` of a set gives the array of its items",
    "file": "src/runtime/collect_options.js",
    "find": "  return into ? into(values) : values;\n",
    "replace": "  return values;\n",
    "tests": [
      "test/corpus.test.ts",
      "-t",
      "collect_fallible_into"
    ]
  },
];
