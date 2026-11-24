// Mutations of crates/rust-js-program/src/runtime/collect_results.js (ADR 0093).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    "name": "collect-results-not-into",
    "breaks": "`collect()` into a `Result` of a map gives `Ok` of the array of its entries",
    "file": "crates/rust-js-program/src/runtime/collect_results.js",
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
    file: "crates/rust-js-program/src/runtime/collect_results.js",
    find: "  if (drop) $dropItems(values, drop);\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^drops_of_consumed_iterators.rs$"],
  },
  {
    name: "collect-results-unwinds-nothing",
    breaks: "a `collect` into a `Result` a panic stops drops neither what it collected nor what's left (#423)",
    file: "crates/rust-js-program/src/runtime/collect_results.js",
    find: "  }, it, values, drop, restFirst);",
    replace: "  }, {}, values, undefined, restFirst);",
    tests: ["test/corpus.test.ts", "-t", "^panic_part_way_drops.rs$"],
  },
  {
    name: "collect-results-close-holds-nothing",
    breaks: "a `collect` into a `Result` stopped at an `Err` drops nothing it collected where closing what it steps panics (#2564)",
    file: "crates/rust-js-program/src/runtime/collect_results.js",
    find: "    drop && (() => $dropItems(values, drop)),\n",
    replace: "    undefined,\n",
    tests: ["test/corpus.test.ts", "-t", "^fallible_collect_drops_what_it_collected_as_its_close_panics.rs$"],
  },
  {
    name: "collect-results-close-err-unheld",
    breaks: "a `collect` into a `Result` stopped at an `Err` leaves the `Err` it kept undropped where closing what it steps panics (#2564)",
    file: "crates/rust-js-program/src/runtime/collect_results.js",
    find: "    dropErr && (() => dropErr(failed._0)),\n",
    replace: "    undefined,\n",
    tests: ["test/corpus.test.ts", "-t", "^fallible_collect_drops_what_it_collected_as_its_close_panics.rs$"],
  },
];
