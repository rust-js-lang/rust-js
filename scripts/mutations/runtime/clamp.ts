// Mutations of src/runtime/clamp.js (ADR 0093).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "clamp-by-max-first",
    breaks: "`clamp` of text past both bounds takes the lower as the upper",
    file: "src/runtime/clamp.js",
    find: "      return cmp(x, min) < 0 ? 1 : cmp(x, max) > 0 ? 2 : 0;\n",
    replace: "      return cmp(x, min) < 0 ? 1 : cmp(x, max) > 0 ? 1 : 0;\n",
    tests: ["test/corpus.test.ts", "-t", "^(clamp_ord|clamp_char_bounds)\\.rs$"],
  },
  {
    name: "clamp-keeps-the-others",
    breaks: "`clamp` of a type with a destructor drops neither of the two it doesn't give back (#912)",
    file: "src/runtime/clamp.js",
    find: "  if (drop !== undefined) $inTurn(",
    replace: "  if (false) $inTurn(",
    tests: ["test/corpus.test.ts", "-t", "^max_min_clamp_as_std_compares.rs$"],
  },
  {
    name: "clamp-drops-the-first-first",
    breaks: "`clamp` drops the two it doesn't give back the earlier parameter first, where std drops `max`, then `min`, then `self` (#912)",
    file: "src/runtime/clamp.js",
    find: "[2, 1, 0].filter(",
    replace: "[0, 1, 2].filter(",
    tests: ["test/corpus.test.ts", "-t", "^max_min_clamp_as_std_compares.rs$"],
  },
  {
    name: "clamp-leaks-as-it-unwinds",
    breaks: "`clamp` whose comparison panics leaks its bounds, where std drops its parameters as it unwinds (#912)",
    file: "src/runtime/clamp.js",
    find: "    drop && (() => drop(max)),\n    drop && (() => drop(min)),\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^max_min_clamp_as_std_compares.rs$"],
  },
  {
    name: "kept-of-keeps-the-other",
    breaks: "`max` of a type with a destructor never drops the one it doesn't give back (#912)",
    file: "src/runtime/clamp.js",
    find: "  drop(other);\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^max_min_clamp_as_std_compares.rs$"],
  },
  {
    name: "kept-of-unwinds-first-first",
    breaks: "`max` whose comparison panics drops `a` before `b`, where std drops its later parameter first (#912)",
    file: "src/runtime/clamp.js",
    find: "    () => drop(b),\n    () => drop(a),\n",
    replace: "    () => drop(a),\n    () => drop(b),\n",
    tests: ["test/corpus.test.ts", "-t", "^max_min_clamp_as_std_compares.rs$"],
  },
  {
    name: "kept-of-first-against-second",
    breaks: "`max` of a type with a destructor asks `cmp(a, b)`, where std's `max` asks `b < a` (#912)",
    file: "src/runtime/clamp.js",
    find: "    () => cmp(b, a) < 0,\n",
    replace: "    () => cmp(a, b) > 0,\n",
    tests: ["test/corpus.test.ts", "-t", "^max_min_clamp_as_std_compares.rs$"],
  },
];
