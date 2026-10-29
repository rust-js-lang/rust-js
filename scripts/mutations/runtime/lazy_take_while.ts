// Mutations of src/runtime/lazy_take_while.js (ADR 0093).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "take-while-try-fold-by-next",
    breaks: "take_while passes tryFold to its source",
    file: "src/runtime/lazy_take_while.js",
    find: "  taking.tryFold = (init, f, broke, unwind) => {",
    replace: "  taking.tryFoldUnused = (init, f, broke, unwind) => {",
    tests: ["test/corpus.test.ts", "-t", "skip_try_fold"],
  },
  {
    name: "map-while-try-fold-by-next",
    breaks: "map_while passes tryFold to its source",
    file: "src/runtime/lazy_take_while.js",
    find: "  mapping.tryFold = (init, g, broke, unwind) => {",
    replace: "  mapping.tryFoldUnused = (init, g, broke, unwind) => {",
    tests: ["test/corpus.test.ts", "-t", "skip_try_fold"],
  },
  {
    name: "take-while-fold-by-next",
    breaks: "a `take_while`'s fold steps it by `next`, not std's `try_fold`, so a `step_by` inside asks the crate's own `nth(0)` first (#214)",
    file: "src/runtime/lazy_take_while.js",
    find: "    const folded = $consumed(taking, () => taking.tryFold(init, f, () => false, unwind));\n    taking.return();\n    return folded;\n  };\n",
    replace: "    return $Steps.prototype.fold.call(taking, init, f, unwind);\n  };\n",
    tests: ["test/corpus.test.ts", "-t", "^adapters_own_nth_fold.rs$"],
  },
  {
    name: "map-while-fold-by-next",
    breaks: "a `map_while`'s fold steps it by `next`, not std's `try_fold` (#216)",
    file: "src/runtime/lazy_take_while.js",
    find: "    const folded = $consumed(mapping, () => mapping.tryFold(init, g, () => false, unwind));\n",
    replace: "    const folded = $Steps.prototype.fold.call(mapping, init, g, unwind);\n",
    tests: ["test/corpus.test.ts", "-t", "^adapters_own_nth_fold.rs$"],
  },
  {
    name: "take-while-fold-unguarded",
    breaks: "a `take_while` whose predicate panics inside a fold leaves the accumulator undropped (#423)",
    file: "src/runtime/lazy_take_while.js",
    find: "    const kept = unwind === undefined ?",
    replace: "    const kept = true ?",
    tests: ["test/corpus.test.ts", "-t", "^panic_mid_fold_drops.rs$"],
  },
  {
    name: "take-while-fold-unclosed",
    breaks: "a `take_while` whose predicate panics inside a fold leaves what's left undropped (#423)",
    file: "src/runtime/lazy_take_while.js",
    find: "    const folded = $consumed(taking, () => taking.tryFold(init, f, () => false, unwind));",
    replace: "    const folded = taking.tryFold(init, f, () => false, unwind);",
    tests: ["test/corpus.test.ts", "-t", "^panic_mid_fold_drops.rs$"],
  },
  {
    name: "map-while-fold-unguarded",
    breaks: "a `map_while` whose closure panics inside a fold leaves the accumulator undropped (#423)",
    file: "src/runtime/lazy_take_while.js",
    find: "        const value = unwind === undefined ? f(x) :",
    replace: "        const value = true ? f(x) :",
    tests: ["test/corpus.test.ts", "-t", "^panic_mid_fold_drops.rs$"],
  },
  {
    name: "map-while-fold-unclosed",
    breaks: "a `map_while` whose closure panics inside a fold leaves what's left undropped (#423)",
    file: "src/runtime/lazy_take_while.js",
    find: "    const folded = $consumed(mapping, () => mapping.tryFold(init, g, () => false, unwind));",
    replace: "    const folded = mapping.tryFold(init, g, () => false, unwind);",
    tests: ["test/corpus.test.ts", "-t", "^panic_mid_fold_drops.rs$"],
  },
];
