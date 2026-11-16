// The iterator tables' catalog of adapters held to std's own list
// (test/adapter-catalog.test.ts): an adapter std has that no table steps
// is a coverage gap (`filter_map` was the copy matrix's).
import type { Mutation } from "../mutations";
export const mutations: Mutation[] = [
  {
    name: "adapter-catalog-exclusion-forgotten",
    breaks: "an adapter std has and no table steps, `cycle`, isn't said to be excluded, nor why (its exclusion names another): the catalog's check passes over it",
    file: "test/iterator-matrix.ts",
    find: '  cycle: "endless: no table\'s consumer would end; the corpus\'s cycle_* cases step it by `take`",\n',
    replace: '  cyclic: "endless: no table\'s consumer would end; the corpus\'s cycle_* cases step it by `take`",\n',
    tests: ["test/adapter-catalog.test.ts"],
  },
];
