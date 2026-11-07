// Mutations of src/runtime/repeat.js (ADR 0093).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "repeat-none-undropped",
    breaks: "`vec![x; 0]` of a value with a destructor never drops `x`",
    file: "src/runtime/repeat.js",
    find: "  } else {\n    drop?.(item);\n",
    replace: "  } else {\n",
    tests: ["test/corpus.test.ts", "-t", "^vec_repeat_drops.rs$"],
  },
  {
    name: "repeat-no-room-undropped",
    breaks: "`vec![x; n]` with no room for them leaves `x` undropped",
    file: "src/runtime/repeat.js",
    find: "    () => drop?.(item),\n  );\n",
    replace: "    () => undefined,\n  );\n",
    tests: ["test/corpus.test.ts", "-t", "^capacity_overflow.rs$"],
  },
  {
    name: "repeat-dropping-room-unchecked",
    breaks: "`vec![x; n]` of items with a destructor past `isize::MAX` bytes is no panic",
    file: "src/runtime/repeat.js",
    find: "    () => $roomFor(count, size),\n",
    replace: "    () => undefined,\n",
    tests: ["test/runtime-package.test.ts", "-t", "room past"],
  },
  {
    name: "repeat-clone-keeps-value",
    breaks: "`vec![x; n]` whose clone of `x` panics leaves `x` undropped (#898)",
    file: "src/runtime/repeat.js",
    find: "      () => drop?.(item),\n      () => drop && $dropItems(result, drop),\n",
    replace: "      () => drop && $dropItems(result, drop),\n",
    tests: ["test/corpus.test.ts", "-t", "^resize_fill_repeat_unwind.rs$"],
  },
  {
    name: "repeat-clone-keeps-clones",
    breaks: "`vec![x; n]` whose clone of `x` panics leaves the clones it made undropped (#898)",
    file: "src/runtime/repeat.js",
    find: "      () => drop && $dropItems(result, drop),\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^resize_fill_repeat_unwind.rs$"],
  },
];
