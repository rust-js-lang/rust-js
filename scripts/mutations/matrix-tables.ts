// The tables as the iterator matrix's harness runs them (test/matrix-tables.ts):
// a change's picked rows in programs of the rows' owner's size (#2340).
import type { Mutation } from "../mutations";
export const mutations: Mutation[] = [
  {
    name: "rows-picked-in-programs-of-60",
    breaks: "a change's picked rows run in programs of 60, a compile's start and rustc's paid four times as often: #2331's 90,113 matrix rows in 1,566 programs, 514–673 s of check:fast (#2340)",
    file: "test/matrix-tables.ts",
    find: "  batches: (all, picked) => batches(all, picked ? pickedPerBatch : 60, aborts),\n",
    replace: "  batches: (all, picked) => batches(all, picked ? 60 : 60, aborts),\n",
    tests: ["test/iterator-rows.test.ts", "-t", "a change's rows run in programs of the rows' owner's size"],
  },
];
