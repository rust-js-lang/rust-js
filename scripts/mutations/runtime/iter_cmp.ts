// Mutations of src/runtime/iter_cmp.js (ADR 0093).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "iter-cmp-shorter-greater",
    breaks: "an iterator that ends before another compares `Greater`, where it's `Less`",
    file: "src/runtime/iter_cmp.js",
    find: "  return y.next().done ? 0 : -1;\n",
    replace: "  return y.next().done ? 0 : 1;\n",
    tests: ["test/corpus.test.ts", "-t", "^iter_compare.rs$"],
  },
  {
    name: "iter-cmp-by-next",
    breaks: "cmp steps its left side by tryFold",
    file: "src/runtime/iter_cmp.js",
    find: "  const stopped = $lent(x).tryFold(",
    replace: "  const stopped = new $Steps(() => x.next()).tryFold(",
    tests: ["test/corpus.test.ts", "-t", "skip_try_fold"],
  },
  {
    name: "iter-cmp-left-open",
    breaks: "`cmp` never drops what either iterator has left, `iter_compare` not closing what it took (#242)",
    file: "src/runtime/iter_cmp.js",
    find: "  return $endsAfter(() => $closeEach(y, x), () => {\n    let order",
    replace: "  return $endsAfter(undefined, () => {\n    let order",
    tests: ["test/corpus.test.ts", "-t", "^consumer_drops_what_is_left.rs$"],
  },
  {
    name: "iter-cmp-close-stops-at-a-panic",
    breaks: "`cmp` stops closing where its second iterator's drop panics, so the first's rest is never dropped (#743)",
    file: "src/runtime/iter_cmp.js",
    find: "  return $endsAfter(() => $closeEach(y, x), () => {\n    let order",
    replace: "  return $endsAfter(() => {\n    y.return?.();\n    x.return?.();\n  }, () => {\n    let order",
    tests: ["test/corpus.test.ts", "-t", "^parts_close_past_a_panic.rs$"],
  },
  {
    name: "compared-unheld",
    breaks:
      "an iterator `eq` or `cmp` compares is dropped as its step ends, the crate's own with a `Drop` before the item taken of the other, not at the comparison's end as std's `iter_compare` drops it (#1145)",
    file: "src/runtime/iter_cmp.js",
    find: "  steps.hold?.();\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^iterator_comparisons_drop_as_std.rs$"],
  },
];
