// Mutations of src/runtime/try_fold_some.js (ADR 0458).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "try-fold-some-goes-on",
    breaks: "`try_fold` into an `Option` goes on past a `None`",
    file: "src/runtime/try_fold_some.js",
    find: "    if (made == null) return made;\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^try_fold.rs$"],
  },
  {
    name: "try-fold-some-steps-one-more",
    breaks: "`try_fold` into an `Option` that stops steps one item more, which the caller doesn't get",
    file: "src/runtime/try_fold_some.js",
    find: "    if (made == null) return made;\n",
    replace: "    if (made == null) return (it.next(), made);\n",
    tests: ["test/corpus.test.ts", "-t", "^try_fold.rs$"],
  },
];
