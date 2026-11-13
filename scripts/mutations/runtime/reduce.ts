// Mutations of src/runtime/reduce.js (ADR 0479).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "reduce-of-none-js",
    breaks: "`reduce` of no items is JS's, which throws",
    file: "src/runtime/reduce.js",
    find: "  if (first.done) {\n    it.return?.();\n    return undefined;\n  }\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^iterator_reduce.rs$"],
  },
  {
    name: "reduce-empty-left-open",
    breaks: "`reduce` of none never drops what its source has left, its `next()?` not closing it (#242)",
    file: "src/runtime/reduce.js",
    find: "  if (first.done) {\n    it.return?.();\n    return undefined;",
    replace: "  if (first.done) {\n    return undefined;",
    tests: ["test/corpus.test.ts", "-t", "^consumer_drops_what_is_left.rs$"],
  },
  {
    name: "reduce-unwind-lost",
    breaks: "a `reduce` of items with a destructor gives its fold no `unwind` (#423)",
    file: "src/runtime/reduce.js",
    find: "  const value = $foldDropping(it, first.value, f, unwind);",
    replace: "  const value = $foldDropping(it, first.value, f);",
    tests: ["test/corpus.test.ts", "-t", "^panic_mid_fold_drops.rs$"],
  },
  {
    name: "reduce-by-drops-what-it-gave-back",
    breaks: "`max()` of the crate's iterator of hand-ordered items whose `Drop` panics as the fold ends drops the item it kept, where std's fold has given it back and leaks it (#1655's rule; ADR 0109, amended)",
    file: "src/runtime/reduce.js",
    find: "  return $reduce(it, (kept, item) => extreme(kept, item), boxed, drop);\n",
    replace: "  let held;\n  return $unwinds(\n    () => $reduce(it, (kept, item) => (held = extreme(kept, item)), boxed, drop),\n    () => held !== undefined && drop?.(held),\n  );\n",
    tests: ["test/corpus.test.ts", "-t", "^reduce_leaks_what_it_kept_as_its_iterators_drop_panics.rs$"],
  },
];
