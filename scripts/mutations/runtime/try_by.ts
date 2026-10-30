// Mutations of src/runtime/try_by.js (ADR 0093).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "control-flow-try-never-breaks",
    breaks: "a `ControlFlow`'s `Break` reads as its `Continue`, so std's `Take::try_fold` steps past its count (ADR 0913)",
    file: "src/runtime/try_by.js",
    find: "  branch: (flow) => (flow.TAG === \"Continue\" ? flow : { TAG: \"Break\", _0: flow }),\n",
    replace: "  branch: (flow) => ({ TAG: \"Continue\", _0: flow }),\n",
    tests: ["test/corpus.test.ts", "-t", "^held_skip_take.rs$"],
  },
  {
    name: "tried-of-never-breaks",
    breaks: "what a JS iterator's `tryFold` folds into never breaks when std's compiled `try_fold` asks, so `try_fold` of a held `take` steps on past a `None` (ADR 0913)",
    file: "src/runtime/try_by.js",
    find: "    branch: (made) => (broke(made) ? { TAG: \"Break\", _0: made } : { TAG: \"Continue\", _0: made }),\n",
    replace: "    branch: (made) => ({ TAG: \"Continue\", _0: made }),\n",
    tests: ["test/iterator-forms.test.ts", "-t", "^held_skip_take with std's iterator structs$"],
  },
  {
    name: "try-fold-into-breaks-on-continue",
    breaks: "`try_fold` into a `ControlFlow` stops at its first `Continue`, so std's `Take::fold` folds one item (ADR 0913)",
    file: "src/runtime/try_by.js",
    find: "    (made) => tried.branch(made).TAG === \"Break\",\n",
    replace: "    (made) => tried.branch(made).TAG === \"Continue\",\n",
    tests: ["test/corpus.test.ts", "-t", "^held_skip_take.rs$"],
  },
  {
    name: "try-fold-into-unwind-lost",
    breaks: "a `try_fold` into an `R` of its `Try`, `TakeWhile::fold`'s, gives what it folds no `unwind`: a stage's panic leaves the accumulator undropped (#423)",
    file: "src/runtime/try_by.js",
    find: "    unwind && ((made) => unwind(tried.branch(made)._0)),\n",
    replace: "",
    tests: ["test/corpus.test.ts", "test/iterator-forms.test.ts", "-t", "^generic_folds_unwind.rs$|^panic_mid_fold_drops with"],
  },
];
