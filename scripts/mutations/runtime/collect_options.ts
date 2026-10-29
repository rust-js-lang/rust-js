// Mutations of src/runtime/collect_options.js (ADR 0093).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    "name": "collect-options-not-into",
    "breaks": "`collect()` into an `Option` of a set gives the array of its items",
    "file": "src/runtime/collect_options.js",
    "find": "  if (!failed) return into ? into(values) : values;\n",
    "replace": "  if (!failed) return values;\n",
    "tests": [
      "test/corpus.test.ts",
      "-t",
      "collect_fallible_into"
    ]
  },
  {
    name: "collect-options-unwinds-nothing",
    breaks: "a `collect` into an `Option` a panic stops drops neither what it collected nor what's left (#423)",
    file: "src/runtime/collect_options.js",
    find: "    $collectUnwound(it, values, drop, restFirst);\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^panic_part_way_drops.rs$"],
  },
];
