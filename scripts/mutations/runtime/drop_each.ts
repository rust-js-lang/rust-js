// Mutations of src/runtime/drop_each.js (ADR 0098, #416).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "drop-items-stop-at-a-throw",
    breaks: "the items after one whose destructor panics are never dropped, `$dropItems` stopping at its throw (#416)",
    file: "src/runtime/drop_each.js",
    find: "  $inTurnOf(items, drop);\n",
    replace: "  for (const item of items) drop(item);\n",
    tests: ["test/corpus.test.ts", "-t", "^drop_parts_past_a_panic.rs$"],
  },
];
