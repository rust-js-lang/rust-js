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
];
