// Mutations of src/js.rs.
import type { Mutation } from "../mutations";

export const mutations: Mutation[] = [
  {
    name: "index-of-one-item-array-kept",
    breaks: "`[x][0]` is written as it is, not `x`",
    file: "src/js.rs",
    find: "            && items.len() == 1\n",
    replace: "            && items.len() == 0\n",
    tests: ["test/corpus.test.ts", "-t", "wrapping_type"],
    snapshots: true,
  },
];
