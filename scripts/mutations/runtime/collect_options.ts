// Mutations of crates/rust-js-program/src/runtime/collect_options.js (ADR 0093).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    "name": "collect-options-not-into",
    "breaks": "`collect()` into an `Option` of a set gives the array of its items",
    "file": "crates/rust-js-program/src/runtime/collect_options.js",
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
    file: "crates/rust-js-program/src/runtime/collect_options.js",
    find: "  }, it, values, drop, restFirst);",
    replace: "  }, {}, values, undefined, restFirst);",
    tests: ["test/corpus.test.ts", "-t", "^panic_part_way_drops.rs$"],
  },
  {
    name: "shunt-hint-panic-drops-not-taken",
    breaks: "where asking the hint of a collect into an `Option` panics, the value just taken isn't dropped, as std's `extend_desugared` drops its element (a wrong answer, #848)",
    file: "crates/rust-js-program/src/runtime/collect_options.js",
    find: "        () => drop?.(value),\n",
    replace: "        () => undefined,\n",
    tests: ["test/corpus.test.ts", "-t", "^collect_into_results_asks_hint.rs$"],
  },
  {
    name: "collect-options-close-holds-nothing",
    breaks: "a `collect` into an `Option` stopped at a `None` drops nothing it collected where closing what it steps panics (#2564)",
    file: "crates/rust-js-program/src/runtime/collect_options.js",
    find: "  $closeHolding(it, drop && (() => $dropItems(values, drop)));\n",
    replace: "  $closeBare(it);\n",
    tests: ["test/corpus.test.ts", "-t", "^fallible_collect_drops_what_it_collected_as_its_close_panics.rs$"],
  },
];
