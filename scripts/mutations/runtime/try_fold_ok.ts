// Mutations of src/runtime/try_fold_ok.js (ADR 0458).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "try-fold-ok-goes-on",
    breaks: "`try_fold` into a `Result` goes on past an `Err`",
    file: "src/runtime/try_fold_ok.js",
    find: "    if (made.TAG === \"Err\") return made;\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^try_fold.rs$"],
  },
  {
    name: "try-fold-ok-of-nothing",
    breaks: "`try_fold` into a `Result` of no items gives its start, not an `Ok` of it",
    file: "src/runtime/try_fold_ok.js",
    find: "  let made = { TAG: \"Ok\", _0: init };\n",
    replace: "  let made = init;\n",
    tests: ["test/corpus.test.ts", "-t", "^try_fold.rs$"],
  },
];
