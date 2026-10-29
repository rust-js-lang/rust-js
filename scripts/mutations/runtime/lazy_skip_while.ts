// Mutations of src/runtime/lazy_skip_while.js (ADR 0093).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "skip-while-try-fold-by-next",
    breaks: "skip_while's tryFold is its first step, then its source's",
    file: "src/runtime/lazy_skip_while.js",
    find: "    return $lent(source).tryFold(folded, f, broke, unwind);",
    replace: "    for (let step = source.next(); !step.done; step = source.next()) {\n      folded = f(folded, step.value);\n      if (broke(folded)) return folded;\n    }\n    return folded;",
    tests: ["test/corpus.test.ts", "-t", "skip_try_fold"],
  },
  {
    name: "skip-while-next-by-next",
    breaks: "skip_while's next is its source's find",
    file: "src/runtime/lazy_skip_while.js",
    find: "      const found = $lent(source).tryFold(",
    replace: "      const found = new $Steps(() => source.next()).tryFold(",
    tests: ["test/corpus.test.ts", "-t", "skip_try_fold"],
  },
  {
    name: "skip-while-fold-by-next",
    breaks: "a `skip_while`'s fold steps it by `next`, not std's first step then its source's own fold (#214)",
    file: "src/runtime/lazy_skip_while.js",
    find: "      return source.fold !== undefined ? source.fold(folded, f, unwind) : source.reduce(f, folded);\n",
    replace: "      return $Steps.prototype.fold.call(skipped, folded, f, unwind);\n",
    tests: ["test/corpus.test.ts", "-t", "^adapters_own_nth_fold.rs$"],
  },
  {
    name: "skip-while-fold-left-open",
    breaks: "`take(1).skip_while(..).count()` never drops what the `take`'s source has left, its fold's first step finding none (#242)",
    file: "src/runtime/lazy_skip_while.js",
    find: "        if (step.done) {\n          skipped.return();\n          return folded;\n        }\n",
    replace: "        if (step.done) {\n          return folded;\n        }\n",
    tests: ["test/corpus.test.ts", "-t", "^consumer_drops_what_is_left.rs$"],
  },
  {
    name: "skip-while-first-unguarded",
    breaks: "a `skip_while` whose predicate panics on its first step inside a fold leaves the accumulator undropped (#423)",
    file: "src/runtime/lazy_skip_while.js",
    find: "    if (unwind === undefined) return skipped.next();",
    replace: "    if (true) return skipped.next();",
    tests: ["test/corpus.test.ts", "-t", "^panic_mid_fold_drops.rs$"],
  },
  {
    name: "skip-while-fold-unclosed",
    breaks: "a `skip_while` whose predicate panics inside a fold leaves what's left undropped (#423)",
    file: "src/runtime/lazy_skip_while.js",
    find: "  skipped.fold = (init, f, unwind) =>\n    $consumed(skipped, () => {",
    replace: "  skipped.fold = (init, f, unwind) =>\n    ((run) => run())(() => {",
    tests: ["test/corpus.test.ts", "-t", "^panic_mid_fold_drops.rs$"],
  },
  {
    name: "skip-while-fold-unwind-lost",
    breaks: "a `skip_while` drops the `unwind` its fold is given (#423)",
    file: "src/runtime/lazy_skip_while.js",
    find: "source.fold(folded, f, unwind) : source.reduce",
    replace: "source.fold(folded, f) : source.reduce",
    tests: ["test/corpus.test.ts", "-t", "^panic_mid_fold_drops.rs$"],
  },
];
