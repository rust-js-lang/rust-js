// Mutations of src/runtime/lent.js (ADR 0071, #162).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "lent-nth-steps",
    breaks: "`nth` of a `&mut` of the crate's iterator steps it by `next`, not by its own `nth`, which std's `&mut I` gives it",
    file: "src/runtime/lent.js",
    find: "  if (it.nth !== undefined) lent.nth = (k) => it.nth(k);\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^own_fold_and_nth.rs$"],
  },
];
