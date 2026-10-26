// Mutations of src/runtime/lent.js (ADR 0071, #162).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "lent-nth-drop-unpassed",
    breaks: "a lent iterator's `nth` doesn't pass on the items' drop, so a chain's `nth` past a compiled `fuse()`'s items never drops them (#285)",
    file: "src/runtime/lent.js",
    find: "  if (it.nth !== undefined) lent.nth = (k, drop) => it.nth(k, drop);\n",
    replace: "  if (it.nth !== undefined) lent.nth = (k) => it.nth(k);\n",
    tests: ["test/corpus.test.ts", "-t", "^chain_sides_own_steps.rs$"],
  },
  {
    name: "lent-nth-steps",
    breaks: "`nth` of a `&mut` of the crate's iterator steps it by `next`, not by its own `nth`, which std's `&mut I` gives it",
    file: "src/runtime/lent.js",
    find: "  if (it.nth !== undefined) lent.nth = (k, drop) => it.nth(k, drop);\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^own_fold_and_nth.rs$"],
  },
  {
    name: "lent-try-fold-own",
    breaks: "a lent iterator's tryFold is its iterator's",
    file: "src/runtime/lent.js",
    find: "  if (it.tryFold !== undefined) lent.tryFold",
    replace: "  if (false) lent.tryFold",
    tests: ["test/corpus.test.ts", "-t", "skip_try_fold"],
  },
  {
    name: "lent-struct-fields-unread",
    breaks: "a compiled std struct lent by `by_ref()` hasn't its fields, `iter` and `count`, which its compiled bodies read",
    file: "src/runtime/lent.js",
    find: "  if (it.$steps !== undefined) {\n",
    replace: "  if (false) {\n",
    tests: ["test/corpus.test.ts", "-t", "^std_enumerate.rs$"],
  },
];
