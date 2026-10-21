// Mutations of src/runtime/try_fold_some.js (ADR 0458).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "try-fold-some-goes-on",
    breaks: "`try_fold` into an `Option` goes on past a `None`",
    file: "src/runtime/try_fold_some.js",
    find: "    if (some == null) return undefined;\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^try_fold.rs$"],
  },
  {
    name: "try-fold-some-steps-one-more",
    breaks: "`try_fold` into an `Option` that stops steps one item more, which the caller doesn't get",
    file: "src/runtime/try_fold_some.js",
    find: "    if (some == null) return undefined;\n",
    replace: "    if (some == null) return (it.next(), undefined);\n",
    tests: ["test/corpus.test.ts", "-t", "^try_fold.rs$"],
  },
  {
    name: "try-fold-some-boxed-made",
    breaks: "`try_fold` into an `Option` of what could look like `None` gives its function what's made boxed",
    file: "src/runtime/try_fold_some.js",
    find: "    made = $someValue(some);\n",
    replace: "    made = some;\n",
    tests: ["test/corpus.test.ts", "-t", "^try_for_each.rs$"],
  },
  {
    name: "try-fold-some-unboxed-result",
    breaks: "`try_fold` into an `Option` of what could look like `None` gives `Some(None)` as `None`",
    file: "src/runtime/try_fold_some.js",
    find: "  return $some(made);\n",
    replace: "  return made;\n",
    tests: ["test/corpus.test.ts", "-t", "^try_for_each.rs$"],
  },
];
