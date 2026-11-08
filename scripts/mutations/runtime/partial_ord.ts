// Mutations of src/runtime/partial_ord.js (ADR 0093).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "partial-ord-own-lt-replaced",
    breaks: "a `PartialOrd` dictionary's own `lt` is replaced by std's, so generic `<` never runs an impl's (#944)",
    file: "src/runtime/partial_ord.js",
    find: "  dictionary.lt ??= (a, b) => partialCmp(a, b) < 0;\n",
    replace: "  dictionary.lt = (a, b) => partialCmp(a, b) < 0;\n",
    tests: ["test/corpus.test.ts", "-t", "^generic_compare_by_impls.rs$"],
  },
  {
    name: "partial-ord-le-strict",
    breaks: "std's `le` of a `PartialOrd` dictionary is its `<`, so a heap or generic `<=` of equal items says false (#944)",
    file: "src/runtime/partial_ord.js",
    find: "  dictionary.le ??= (a, b) => partialCmp(a, b) <= 0;\n",
    replace: "  dictionary.le ??= (a, b) => partialCmp(a, b) < 0;\n",
    tests: ["test/corpus.test.ts", "-t", "^(generic_compare_by_impls|heap_methods|max_min_clamp_as_std_compares)\\.rs$"],
  },
];
