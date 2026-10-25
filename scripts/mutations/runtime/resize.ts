// Mutations of src/runtime/resize.js (ADR 0093).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "resize-moves-no-original",
    breaks: "the value given isn't the last item, cloned like the rest",
    file: "src/runtime/resize.js",
    find: "  v.push(item);\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "vec_edits"],
  },
  {
    name: "resize-keeps-value",
    breaks: "`resize` that shortens drops none of the value it's given",
    file: "src/runtime/resize.js",
    find: "    $truncate(v, length, drop);\n    drop?.(item);\n",
    replace: "    $truncate(v, length, drop);\n",
    tests: ["test/corpus.test.ts", "-t", "^slice_edit_drops.rs$"],
  },
  {
    name: "resize-room-keeps-value",
    breaks: "`resize` of no room doesn't drop the value it's given as it unwinds",
    file: "src/runtime/resize.js",
    find: "      drop?.(item);\n      throw panic;\n",
    replace: "      throw panic;\n",
    tests: ["test/corpus.test.ts", "-t", "^asked_length_overflow.rs$"],
  },
];
