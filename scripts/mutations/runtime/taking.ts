// Mutations of src/runtime/taking.js (ADR 0542).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "taking-back-uncounted",
    breaks: "an explicit `take` stepped from its back doesn't count down, so it gives more than it takes",
    file: "src/runtime/taking.js",
    find: "    const past = Math.max(taking.iter.len() - taking.n, 0);\n    taking.n--;\n",
    replace: "    const past = Math.max(taking.iter.len() - taking.n, 0);\n",
    tests: ["test/corpus.test.ts", "-t", "^(own_back_through_adapters).rs$"],
  },
  {
    name: "taking-length-its-source",
    breaks: "an explicit `take`'s length is its source's, not the fewer of that and how many it takes",
    file: "src/runtime/taking.js",
    find: "  taking.len = () => Math.min(taking.iter.len(), taking.n);\n",
    replace: "  taking.len = () => taking.iter.len();\n",
    tests: ["test/corpus.test.ts", "-t", "^(held_back_folds_by_try_rfold|own_back_through_adapters).rs$"],
  },
  {
    name: "taking-front-uncounted",
    breaks: "an explicit `take` stepped from its front never stops",
    file: "src/runtime/taking.js",
    find: "      taking.n--;\n      return taking.iter.next();\n",
    replace: "      return taking.iter.next();\n",
    tests: ["test/corpus.test.ts", "-t", "^counted_adapters_given.rs$"],
  },
  {
    name: "taking-back-by-next",
    breaks: "a `take`'s `next_back` steps its source back by `next_back`, not its `nth_back` past what it won't give (#220)",
    file: "src/runtime/taking.js",
    find: "    return $nthBackOf(taking.iter, past, drop);\n  };\n  taking.len",
    replace: "    for (let k = 0; k < past; k++) taking.iter.back();\n    return taking.iter.back();\n  };\n  taking.len",
    tests: ["test/corpus.test.ts", "-t", "^own_back_through_adapters.rs$"],
  },
  {
    name: "taking-rfold-untrimmed",
    breaks: "a `take`'s `rfold` folds its source untrimmed, not after its `nth_back` past what it won't give (#220)",
    file: "src/runtime/taking.js",
    find: "      const folded = taking.n === 0 || !trimmed() ? init : $foldBack(taking.iter, init, f, unwind, ends);\n",
    replace: "      const folded = taking.n === 0 ? init : $foldBack(taking.iter, init, f, unwind, ends);\n",
    tests: ["test/corpus.test.ts", "-t", "^own_back_through_adapters.rs$"],
  },
  {
    name: "taking-nth-back-unasked",
    breaks: "a `take`'s `nth_back` steps by `next_back`, not its source's `nth_back` (#220)",
    file: "src/runtime/taking.js",
    find: "    $forward(taking, iter, { nthBack, rfold, tryRfold });\n",
    replace: "    $forward(taking, iter, { rfold, tryRfold });\n",
    tests: ["test/corpus.test.ts", "-t", "^own_back_through_adapters.rs$"],
  },
  {
    name: "taking-passed-undropped",
    breaks:
      "what a `take`'s source's `nth_back` past what it won't give finds, which std's `Take::rfold` and `nth_back` let go of, isn't dropped: `ds().into_iter().take(3).rev().sum()` never drops `D(4)` (#718)",
    file: "src/runtime/taking.js",
    find: "      drop?.(step.value);\n      return true;\n",
    replace: "      return true;\n",
    tests: ["test/corpus.test.ts", "-t", "^skip_take_of_droppers_from_back.rs$"],
  },
  {
    name: "taking-nth-back-past-undropped",
    breaks:
      "a `take`'s `nth_back` past its end lets what its source's `nth_back` finds go undropped, where std's drops it: `take(3).rev().nth(5)` of a `Vec` of five never drops `D(1)` (#718)",
    file: "src/runtime/taking.js",
    find: "      if (len > 0) passed(len - 1);\n",
    replace: "      if (len > 0) $nthBackOf(taking.iter, len - 1, drop);\n",
    tests: ["test/corpus.test.ts", "-t", "^skip_take_of_droppers_from_back.rs$"],
  },
];
