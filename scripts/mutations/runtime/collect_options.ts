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
    find: "  }, it, values, drop, restFirst);",
    replace: "  }, {}, values, undefined, restFirst);",
    tests: ["test/corpus.test.ts", "-t", "^panic_part_way_drops.rs$"],
  },
  {
    name: "shunt-hint-panic-drops-not-taken",
    breaks: "where asking the hint of a collect into an `Option` panics, the value just taken isn't dropped, as std's `extend_desugared` drops its element (a wrong answer, #848)",
    file: "src/runtime/collect_options.js",
    find: "        () => drop?.(value),\n",
    replace: "        () => undefined,\n",
    tests: ["test/corpus.test.ts", "-t", "^collect_into_results_asks_hint.rs$"],
  },
];
