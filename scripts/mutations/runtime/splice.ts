// Mutations of src/runtime/splice.js (ADR 0093).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "splice-replaces-nothing",
    breaks: "`splice` removes the range but puts nothing in its place",
    file: "src/runtime/splice.js",
    find: "  $extend(items, added);\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "vec_edits"],
  },
  {
    name: "splice-replacement-spread",
    breaks: "a splice's replacement, any iterable, is spread into `splice`'s arguments, which Babel's loose spread takes as one item, and more of which than a JS call takes is a `RangeError` (ADR 0919)",
    file: "src/runtime/splice.js",
    find: "  const added = Array.from(replacement);\n  const tail = items.splice(end);\n  const removed = items.splice(start);\n  $extend(items, added);\n  $extend(items, tail);\n  return removed;\n",
    replace: "  return items.splice(start, end - start, ...replacement);\n",
    tests: ["test/runtime-transpiled.test.ts"],
  },
];
