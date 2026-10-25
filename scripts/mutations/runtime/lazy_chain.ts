// Mutations of src/runtime/lazy_chain.js (ADR 0465).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "chain-second-unclosed",
    breaks: "`chain`'s second side isn't closed with it, so none of it is dropped",
    file: "src/runtime/lazy_chain.js",
    find: "      second.return?.();\n    },\n",
    replace: "    },\n",
    tests: ["test/corpus.test.ts", "-t", "^adapter_drops.rs$"],
  },
  {
    name: "chain-fold-steps",
    breaks: "`chain`'s fold steps each side by `next`, not by its own `fold`, `a`'s then `b`'s",
    file: "src/runtime/lazy_chain.js",
    find: "  chained.fold = (init, f) => {\n",
    replace: "  chained.unused = (init, f) => {\n",
    tests: ["test/corpus.test.ts", "-t", "^own_fold_and_nth.rs$"],
  },
  {
    name: "chain-try-fold-by-next",
    breaks: "a chain's tryFold is a's then b's",
    file: "src/runtime/lazy_chain.js",
    find: "  chained.tryFold = (init, f, broke) => {",
    replace: "  chained.tryFoldUnused = (init, f, broke) => {",
    tests: ["test/corpus.test.ts", "-t", "skip_try_fold"],
  },
  {
    name: "chain-last-first-unasked",
    breaks: "a chain's `last` asks only its second side's `last`, not its first's",
    file: "src/runtime/lazy_chain.js",
    find: "    const a = first !== undefined ? last(first) : { done: true, value: undefined };\n",
    replace: "    const a = { done: true, value: undefined };\n",
    tests: ["test/corpus.test.ts", "-t", "^own_last_and_nth_passed_on.rs$"],
  },
  {
    name: "chain-nth-by-next",
    breaks: "a chain's `nth` steps it by `next`, not std's: its first side advanced, then its second's own `nth` (#214)",
    file: "src/runtime/lazy_chain.js",
    find: "  chained.nth = (n) => {\n",
    replace: "  chained.nthUnused = (n) => {\n",
    tests: ["test/corpus.test.ts", "-t", "^adapters_own_nth_fold.rs$"],
  },
];
