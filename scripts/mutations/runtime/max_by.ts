// Mutations of src/runtime/max_by.js (ADR 0464).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "extreme-compare-reversed",
    breaks: "`max_by`'s `compare` is given the item first, `compare(item, kept)`, not std's `compare(kept, item)`",
    file: "src/runtime/max_by.js",
    find: "    } else if ((compare(kept, item) > 0) === max) {\n",
    replace: "    } else if ((compare(item, kept) < 0) === max) {\n",
    tests: ["test/corpus.test.ts", "-t", "^extreme_drops.rs$"],
  },
  {
    name: "extreme-ties-earlier",
    breaks: "`max_by` keeps the earlier of two equal items, where std keeps the later",
    file: "src/runtime/max_by.js",
    find: "    } else if ((compare(kept, item) > 0) === max) {\n",
    replace: "    } else if ((compare(kept, item) >= 0) === max) {\n",
    tests: ["test/corpus.test.ts", "-t", "^extreme_drops.rs$"],
  },
  {
    name: "extreme-passed-over-kept",
    breaks: "an item `max_by` passes over, later than what it keeps, isn't dropped",
    file: "src/runtime/max_by.js",
    find: "      drop?.(item);\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^extreme_drops.rs$"],
  },
  {
    name: "extreme-by-key-per-compare",
    breaks: "`max_by_key`'s key is made again for each comparison, not once for each item as std's",
    file: "src/runtime/max_by.js",
    find: "  const pair = $extremeBy(keyed, (a, b) => compare(a[0], b[0]), max, false, drop && ((pair) => drop(pair[1])));\n",
    replace: "  const pair = $extremeBy(keyed, (a, b) => compare(key(a[1]), key(b[1])), max, false, drop && ((pair) => drop(pair[1])));\n",
    tests: ["test/corpus.test.ts", "-t", "^extreme_drops.rs$"],
  },
];
