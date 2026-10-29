// Mutations of src/runtime/truncate.js (ADR 0098).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "truncate-drops-none",
    breaks: "`truncate` and `clear` of items with a destructor drop none of what they take out",
    file: "src/runtime/truncate.js",
    find: "    if (drop !== undefined) $dropItems(gone, drop);\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^vec_destructors.rs$"],
  },
  {
    name: "truncate-stops-at-a-panic",
    breaks: "`truncate` and `clear` stop at an item whose destructor panics, the rest never dropped, where std's slice drop goes on (#416)",
    file: "src/runtime/truncate.js",
    find: "    if (drop !== undefined) $dropItems(gone, drop);\n",
    replace: "    if (drop !== undefined) for (const item of gone) drop(item);\n",
    tests: ["test/corpus.test.ts", "-t", "^drop_parts_past_a_panic.rs$"],
  },
];
