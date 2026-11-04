// Mutations of src/runtime/try_fold_ok.js (ADR 0458).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "try-fold-ok-goes-on",
    breaks: "`try_fold` into a `Result` goes on past an `Err`",
    file: "src/runtime/try_fold_ok.js",
    find: "    (made) => made.TAG === \"Err\",\n",
    replace: "    (made) => false,\n",
    tests: ["test/corpus.test.ts", "-t", "^try_fold.rs$"],
  },
  {
    name: "try-fold-ok-of-nothing",
    breaks: "`try_fold` into a `Result` of no items gives its start, not an `Ok` of it",
    file: "src/runtime/try_fold_ok.js",
    find: "    { TAG: \"Ok\", _0: init },\n",
    replace: "    init,\n",
    tests: ["test/corpus.test.ts", "-t", "^try_fold.rs$"],
  },
  {
    name: "try-fold-ok-by-next",
    breaks: "try_fold into a Result is its iterator's tryFold",
    file: "src/runtime/try_fold_ok.js",
    find: "  return $lent(it).tryFold(\n    { TAG: \"Ok\"",
    replace: "  return new $Steps(() => it.next()).tryFold(\n    { TAG: \"Ok\"",
    tests: ["test/corpus.test.ts", "-t", "skip_try_fold"],
  },
  {
    name: "try-fold-ok-unwind-lost",
    breaks: "a `try_fold` into a `Result` gives what it folds no `unwind`, so the accumulator a panicking step strands is never dropped (rd767)",
    file: "src/runtime/try_fold_ok.js",
    find: "    unwind && ((made) => unwind(made._0)),\n",
    replace: "    undefined,\n",
    tests: ["test/corpus.test.ts", "-t", "^adapters_over_own_next_that_panics.rs$"],
  },
];
