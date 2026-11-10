// Mutations of src/runtime/truncate.js (ADR 0098).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "truncate-drops-none",
    breaks: "`truncate` and `clear` of items with a destructor drop none of what they take out",
    file: "src/runtime/truncate.js",
    find: "    // of the slice goes on (#416).\n    if (drop !== undefined) $dropItems(gone, drop);\n",
    replace: "    // of the slice goes on (#416).\n",
    tests: ["test/corpus.test.ts", "-t", "^vec_destructors.rs$"],
  },
  {
    name: "truncate-stops-at-a-panic",
    breaks: "`truncate` and `clear` stop at an item whose destructor panics, the rest never dropped, where std's slice drop goes on (#416)",
    file: "src/runtime/truncate.js",
    find: "    // of the slice goes on (#416).\n    if (drop !== undefined) $dropItems(gone, drop);\n",
    replace: "    // of the slice goes on (#416).\n    if (drop !== undefined) for (const item of gone) drop(item);\n",
    tests: ["test/corpus.test.ts", "-t", "^drop_parts_past_a_panic.rs$"],
  },
  {
    name: "retain-back-drops-none",
    breaks: "a `VecDeque`'s `retain_back` drops nothing of what it takes off its front (#1277)",
    file: "src/runtime/truncate.js",
    find: "    const gone = v.splice(0, v.length - length);\n    if (drop !== undefined) $dropItems(gone, drop);\n",
    replace: "    v.splice(0, v.length - length);\n",
    tests: ["test/corpus.test.ts", "-t", "^deque_retain_back.rs$"],
  },
];
