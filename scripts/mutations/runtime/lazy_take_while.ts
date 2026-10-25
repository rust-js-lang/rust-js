// Mutations of src/runtime/lazy_take_while.js (ADR 0093).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "take-while-try-fold-by-next",
    breaks: "take_while passes tryFold to its source",
    file: "src/runtime/lazy_take_while.js",
    find: "  taking.tryFold = (init, f, broke) => {",
    replace: "  taking.tryFoldUnused = (init, f, broke) => {",
    tests: ["test/corpus.test.ts", "-t", "skip_try_fold"],
  },
  {
    name: "map-while-try-fold-by-next",
    breaks: "map_while passes tryFold to its source",
    file: "src/runtime/lazy_take_while.js",
    find: "  mapping.tryFold = (init, g, broke) => {",
    replace: "  mapping.tryFoldUnused = (init, g, broke) => {",
    tests: ["test/corpus.test.ts", "-t", "skip_try_fold"],
  },
  {
    name: "take-while-fold-by-next",
    breaks: "a `take_while`'s fold steps it by `next`, not std's `try_fold`, so a `step_by` inside asks the crate's own `nth(0)` first (#214)",
    file: "src/runtime/lazy_take_while.js",
    find: "  taking.fold = (init, f) => taking.tryFold(init, f, () => false);\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^adapters_own_nth_fold.rs$"],
  },
];
