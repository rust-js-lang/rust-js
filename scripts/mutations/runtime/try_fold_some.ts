// Mutations of src/runtime/try_fold_some.js (ADR 0458).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "try-fold-some-goes-on",
    breaks: "`try_fold` into an `Option` goes on past a `None`",
    file: "src/runtime/try_fold_some.js",
    find: "(made) => made == null);",
    replace: "(made) => false);",
    tests: ["test/corpus.test.ts", "-t", "^try_fold.rs$"],
  },
  {
    name: "try-fold-some-steps-one-more",
    breaks: "`try_fold` into an `Option` that stops steps one item more, which the caller doesn't get",
    file: "src/runtime/try_fold_some.js",
    find: "(made) => made == null);",
    replace: "(made) => made == null && (it.next(), true));",
    tests: ["test/corpus.test.ts", "-t", "^try_fold.rs$"],
  },
  {
    name: "try-fold-some-boxed-made",
    breaks: "`try_fold` into an `Option` of what could look like `None` gives its function what's made boxed",
    file: "src/runtime/try_fold_some.js",
    find: "(made, x) => f($someValue(made), x)",
    replace: "(made, x) => f(made, x)",
    tests: ["test/corpus.test.ts", "-t", "^try_for_each.rs$"],
  },
  {
    name: "try-fold-some-unboxed-result",
    breaks: "`try_fold` into an `Option` of what could look like `None` gives `Some(None)` as `None`",
    file: "src/runtime/try_fold_some.js",
    find: "tryFold($some(init),",
    replace: "tryFold(init,",
    tests: ["test/corpus.test.ts", "-t", "^(try_for_each|try_fold).rs$"],
  },
  {
    name: "try-fold-some-by-next",
    breaks: "try_fold into an Option is its iterator's tryFold",
    file: "src/runtime/try_fold_some.js",
    find: "  return $lent(it).tryFold($some(init)",
    replace: "  return new $Steps(() => it.next()).tryFold($some(init)",
    tests: ["test/corpus.test.ts", "-t", "skip_try_fold"],
  },
];
