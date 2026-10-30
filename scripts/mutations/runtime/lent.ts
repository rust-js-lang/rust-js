// Mutations of src/runtime/lent.js (ADR 0071, #162).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "lent-back-unpassed",
    breaks: "a JS iterator lent as a `&mut` keeps none of its back, so `r.copied().next_back()` of an `&mut R` throws `it.back is not a function` (ADR 0982)",
    file: "src/runtime/lent.js",
    find: "  if (it.back !== undefined) lent.back = () => it.back();\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^copied_of_a_parameter.rs$"],
  },
  {
    name: "lent-nth-drop-unpassed",
    breaks: "a lent iterator's `nth` doesn't pass on the items' drop, so a chain's `nth` past a compiled `fuse()`'s items never drops them (#285)",
    file: "src/runtime/lent.js",
    find: "  if (it.nth !== undefined) lent.nth = (k, drop) => it.nth(k, drop);\n",
    replace: "  if (it.nth !== undefined) lent.nth = (k) => it.nth(k);\n",
    tests: ["test/corpus.test.ts", "-t", "^(chain_nth_drops_what_it_passes|own_nth_last_dropping).rs$"],
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
    find: "    lent.tryFold = (init, f, broke, unwind) => it.tryFold(init, f, broke, unwind);\n",
    replace: "",
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
  {
    name: "lent-fold-by-next",
    breaks: "a `&mut`'s `fold` steps its lender by `next`, where std's folds by its `try_fold`, an inclusive range's leaving it exhausted at its last (#414)",
    file: "src/runtime/lent.js",
    find: "    lent.fold = (init, f, unwind) => it.tryFold(init, f, () => false, unwind);\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^range_inclusive_exhausted.rs$"],
  },
  {
    name: "lent-try-fold-unwind-lost",
    breaks: "a `&mut`'s `try_fold` gives what it lends no `unwind`: a stage's panic leaves the accumulator undropped (#423)",
    file: "src/runtime/lent.js",
    find: "    lent.tryFold = (init, f, broke, unwind) => it.tryFold(init, f, broke, unwind);\n",
    replace: "    lent.tryFold = (init, f, broke) => it.tryFold(init, f, broke);\n",
    tests: ["test/corpus.test.ts", "test/iterator-forms.test.ts", "-t", "^generic_folds_unwind.rs$|^panic_mid_fold_drops with"],
  },
];
