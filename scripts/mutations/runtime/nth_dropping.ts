// Mutations of src/runtime/nth_dropping.js (ADR 0093).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "nth-dropping-default-stepped",
    breaks: "an `nth` of an adapter without one of its own steps `next`, not std's default `advance_by` by its `try_fold`, so an `inspect` of a reversed `Vec`'s `skip` drops in the wrong order (#302)",
    file: "src/runtime/nth_dropping.js",
    find: "  if (it.nth !== undefined) {\n",
    replace: "  if (it.ownNth) {\n",
    tests: ["test/corpus.test.ts", "-t", "^step_by_compiled_source_nth.rs$"],
  },
  {
    name: "nth-dropping-drop-ungiven",
    breaks: "an `ownNth` iterator isn't given its items' drop, so a chain's `nth` past a compiled `fuse()`'s items never drops them (#285)",
    file: "src/runtime/nth_dropping.js",
    find: "    const step = it.nth(n, drop);\n",
    replace: "    const step = it.nth(n);\n",
    tests: ["test/corpus.test.ts", "-t", "^chain_sides_own_steps.rs$"],
  },
  {
    name: "nth-dropping-keeps-skipped",
    breaks: "`nth` of items with a destructor drops none it skips",
    file: "src/runtime/nth_dropping.js",
    find: "    drop?.(item);\n    n--;\n",
    replace: "    n--;\n",
    tests: ["test/corpus.test.ts", "-t", "^picked_items_drop.rs$"],
  },
];
