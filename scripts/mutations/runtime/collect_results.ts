// Mutations of src/runtime/collect_results.js (ADR 0093).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    "name": "collect-results-not-into",
    "breaks": "`collect()` into a `Result` of a map gives `Ok` of the array of its entries",
    "file": "src/runtime/collect_results.js",
    "find": "  if (failed === undefined) return { TAG: \"Ok\", _0: into ? into(values) : values };\n",
    "replace": "  if (failed === undefined) return { TAG: \"Ok\", _0: values };\n",
    "tests": [
      "test/corpus.test.ts",
      "-t",
      "collect_fallible_into"
    ]
  },
  {
    name: "collect-results-values-undropped",
    breaks: "a `collect` into a `Result` that stops at an `Err` drops none of the values it collected",
    file: "src/runtime/collect_results.js",
    find: "  if (drop) values.forEach((value) => drop(value));\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^drops_of_consumed_iterators.rs$"],
  },
];
