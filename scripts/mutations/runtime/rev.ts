// Mutations of src/runtime/rev.js (ADR 0516).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "rev-unclosed",
    breaks: "a `Rev` dropped drops none of what it holds",
    file: "src/runtime/rev.js",
    find: "    () => it.return?.(),\n",
    replace: "    () => {},\n",
    tests: ["test/corpus.test.ts", "-t", "^rev_drops.rs$"],
  },
  {
    name: "rev-front-first",
    breaks: "`$rev` steps from the front",
    file: "src/runtime/rev.js",
    find: "    () => it.back(),\n",
    replace: "    () => it.next(),\n",
    tests: ["test/corpus.test.ts", "-t", "^(rev_drops|generic_double_ended).rs$"],
  },
  {
    name: "rev-sourceless",
    breaks: "a `$rev` keeps nothing of what it steps for a clone to clone",
    file: "src/runtime/rev.js",
    find: "  rev.source = it;\n",
    replace: "",
    tests: ["test/shipped-crates.test.ts", "-t", "memchr's searches run"],
  },
  {
    name: "rev-fold-by-next",
    breaks: "a `rev()`'s fold steps its source by `next_back`, not the crate's own `rfold` it reaches (#220)",
    file: "src/runtime/rev.js",
    find: "  if (it.rfold !== undefined) rev.fold = (init, f) => it.rfold(init, f);\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^own_back_through_adapters.rs$"],
  },
  {
    name: "rev-nth-by-next",
    breaks: "a `rev()`'s `nth` steps its source by `next_back`, not the crate's own `nth_back` it reaches (#220)",
    file: "src/runtime/rev.js",
    find: "  if (it.nthBack !== undefined) rev.nth = (k) => it.nthBack(k);\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^own_back_through_adapters.rs$"],
  },
  {
    name: "rev-try-fold-by-next",
    breaks: "a `rev()`'s `try_fold` steps its source by `next_back`, not a `take`'s `try_rfold`, which trims by the crate's `nth_back` (#220)",
    file: "src/runtime/rev.js",
    find: "  if (it.tryRfold !== undefined) rev.tryFold = (init, f, broke) => it.tryRfold(init, f, broke);\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^own_back_through_adapters.rs$"],
  },
  {
    name: "rev-advance-by-try-fold",
    breaks: "a `rev()`'s `advance_by` is `$Steps`' of its `try_fold`, not its source's `advance_back_by`, std's of `next_back` (#220)",
    file: "src/runtime/rev.js",
    find: "  rev.advanceBy = (m, drop) => {\n",
    replace: "  rev.unused = (m, drop) => {\n",
    tests: ["test/corpus.test.ts", "-t", "^own_back_through_adapters.rs$"],
  },
  {
    name: "rev-own-nth-unset",
    breaks: "a reversed `Vec`'s `nth` isn't said to drop what it passes over, so `skip` drops each one by one from the back (#234)",
    file: "src/runtime/rev.js",
    find: "  if (it.ownNthBack) rev.ownNth = true;\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^vec_skipped_from_back_drops.rs$"],
  },
];
