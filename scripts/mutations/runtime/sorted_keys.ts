// Mutations of src/runtime/sorted_keys.js (ADR 0093).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "sorted-keys-by-js",
    breaks: "a set's keys are listed by JS's own `sort`, which puts a `None` last without asking the comparator, so a `BTreeSet<Option<_>>` shows it last",
    file: "src/runtime/sorted_keys.js",
    find: "  return $sort(Array.from(set), cmp);\n",
    replace: "  return Array.from(set).sort(cmp);\n",
    tests: ["test/corpus.test.ts", "-t", "^option_sort.rs$"],
  },
];
