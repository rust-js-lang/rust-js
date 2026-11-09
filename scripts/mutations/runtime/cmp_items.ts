// Mutations of src/runtime/cmp_items.js (ADR 0465).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "iter-eq-lengths-unread",
    breaks: "`eq()` of two `Vec`s' iterators of unequal lengths compares their items, dropping them in pairs, where std's `SpecIterEq` gives `false` with none compared (ADR 0992)",
    file: "src/runtime/cmp_items.js",
    find: "    if (lengths !== undefined && lengths[0] !== lengths[1]) return false;\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^eq_of_unequal_lengths.rs$"],
  },
  {
    name: "iter-eq-pair-undropped",
    breaks: "`eq` of items with a destructor drops none of the other side's pairs",
    file: "src/runtime/cmp_items.js",
    find: "      dropB?.(q.value);\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^adapter_drops.rs$"],
  },
  {
    name: "iter-eq-unclosed",
    breaks: "`eq` doesn't close what it compared once it ends, so what's left of either isn't dropped",
    file: "src/runtime/cmp_items.js",
    find: "  return $endsAfter(() => $closeEach(y, x), () => {\n",
    replace: "  return $endsAfter(undefined, () => {\n",
    tests: ["test/corpus.test.ts", "-t", "^adapter_drops.rs$"],
  },
  {
    name: "iter-eq-close-stops-at-a-panic",
    breaks: "`eq` stops closing where its second iterator's drop panics, so the first's rest is never dropped (#743)",
    file: "src/runtime/cmp_items.js",
    find: "  return $endsAfter(() => $closeEach(y, x), () => {\n",
    replace: "  return $endsAfter(() => {\n    y.return?.();\n    x.return?.();\n  }, () => {\n",
    tests: ["test/corpus.test.ts", "-t", "^parts_close_past_a_panic.rs$"],
  },
  {
    name: "iter-eq-by-next",
    breaks: "eq steps its left side by tryFold",
    file: "src/runtime/cmp_items.js",
    find: "    const stopped = $lent(x).tryFold(",
    replace: "    const stopped = new $Steps(() => x.next()).tryFold(",
    tests: ["test/corpus.test.ts", "-t", "skip_try_fold"],
  },
  {
    name: "iter-eq-taken-leaked-at-a-step-panic",
    breaks:
      "`a.eq(b)` whose `b` step panics never drops the item it took of `a`, which std's closure drops as it unwinds (#1145)",
    file: "src/runtime/cmp_items.js",
    find: "        const q = $unwinds(() => y.next(), dropA && (() => dropA(p)));\n",
    replace: "        const q = y.next();\n",
    tests: ["test/corpus.test.ts", "-t", "^iterator_comparisons_drop_as_std.rs$"],
  },
];
