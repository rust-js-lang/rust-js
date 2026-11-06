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
];
