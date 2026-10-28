// Mutations of src/runtime/into_iter.js (ADR 0475).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "into-iter-of-array-js",
    breaks: "`$intoIter` of an array is JS's iterator of it, which knows nothing of where it is",
    file: "src/runtime/into_iter.js",
    find: "  if (Array.isArray(items)) return $iter(items);\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^generic_into_iter_values.rs$"],
  },
  {
    name: "into-iter-of-steps-protocol",
    breaks: "`$intoIter` of a held `$Steps` steps it by JS's own protocol, which closes it at its end, so `FromIterator`'s `items.into_iter()` drops its source at its first `None`, before `summed` (#392)",
    file: "src/runtime/into_iter.js",
    find: "  return items instanceof $Steps ? items : Iterator.from(items);\n",
    replace: "  return Iterator.from(items);\n",
    tests: ["test/corpus.test.ts", "-t", "^held_adapters_stepped_to_their_end.rs$"],
  },
];
