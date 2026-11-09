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
    find: "  const added = $foldToArray(replacement);\n  const tail = items.splice(end);\n  const removed = items.splice(start);\n  $extend(items, added);\n  $extend(items, tail);\n  return removed;\n",
    replace: "  return items.splice(start, end - start, ...replacement);\n",
    tests: ["test/runtime-transpiled.test.ts"],
  },
  {
    name: "splice-replacement-unclosed",
    breaks: "`v.splice(r, it)` lists `it` by `Array.from`, which closes nothing where its step panics (#1141)",
    file: "src/runtime/splice.js",
    find: "  const added = $foldToArray(replacement);\n",
    replace: "  const added = Array.from(replacement);\n",
    tests: ["test/corpus.test.ts", "-t", "^consumer_closes_what_it_steps_on_a_panic.rs$"],
  },
];
