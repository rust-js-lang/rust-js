// Mutations of src/runtime/collect_results.js (ADR 0093).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    "name": "collect-results-not-into",
    "breaks": "`collect()` into a `Result` of a map gives `Ok` of the array of its entries",
    "file": "src/runtime/collect_results.js",
    "find": "  return { TAG: \"Ok\", _0: into ? into(values) : values };\n",
    "replace": "  return { TAG: \"Ok\", _0: values };\n",
    "tests": [
      "test/corpus.test.ts",
      "-t",
      "collect_fallible_into"
    ]
  },
];
