// Mutations of src/runtime/try_fold_ok.js (ADR 0458).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "try-fold-ok-goes-on",
    breaks: "`try_fold` into a `Result` goes on past an `Err`",
    file: "src/runtime/try_fold_ok.js",
    find: "(made) => made.TAG === \"Err\");",
    replace: "(made) => false);",
    tests: ["test/corpus.test.ts", "-t", "^try_fold.rs$"],
  },
  {
    name: "try-fold-ok-of-nothing",
    breaks: "`try_fold` into a `Result` of no items gives its start, not an `Ok` of it",
    file: "src/runtime/try_fold_ok.js",
    find: "tryFold({ TAG: \"Ok\", _0: init },",
    replace: "tryFold(init,",
    tests: ["test/corpus.test.ts", "-t", "^try_fold.rs$"],
  },
  {
    name: "try-fold-ok-by-next",
    breaks: "try_fold into a Result is its iterator's tryFold",
    file: "src/runtime/try_fold_ok.js",
    find: "  return $lent(it).tryFold({",
    replace: "  return new $Steps(() => it.next()).tryFold({",
    tests: ["test/corpus.test.ts", "-t", "skip_try_fold"],
  },
];
