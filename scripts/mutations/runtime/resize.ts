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
    find: "    );\n    drop?.(item);\n    return;\n",
    replace: "    );\n    return;\n",
    tests: ["test/corpus.test.ts", "-t", "^slice_edit_drops.rs$"],
  },
  {
    name: "resize-room-keeps-value",
    breaks: "`resize` of no room, or whose clone panics, doesn't drop the value it's given as it unwinds (#898)",
    file: "src/runtime/resize.js",
    find: "    },\n    () => drop?.(item),\n  );\n  v.push(item);\n",
    replace: "    },\n  );\n  v.push(item);\n",
    tests: ["test/corpus.test.ts", "-t", "^(asked_length_overflow|resize_fill_repeat_unwind).rs$"],
  },
  {
    name: "resize-shorter-unguarded",
    breaks: "`resize` that shortens doesn't drop the value it's given as a drop of what it cuts off panics (#898)",
    file: "src/runtime/resize.js",
    find: "      () => $truncate(v, length, drop),\n      () => drop?.(item),\n",
    replace: "      () => $truncate(v, length, drop),\n",
    tests: ["test/corpus.test.ts", "-t", "^resize_fill_repeat_unwind.rs$"],
  },
];
