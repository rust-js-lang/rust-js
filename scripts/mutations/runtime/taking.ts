// Mutations of src/runtime/taking.js (ADR 0542).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "taking-back-uncounted",
    breaks: "an explicit `take` stepped from its back doesn't count down, so it gives more than it takes",
    file: "src/runtime/taking.js",
    find: "    const past = Math.max(taking.iter.len() - taking.n, 0);\n    taking.n--;\n",
    replace: "    const past = Math.max(taking.iter.len() - taking.n, 0);\n",
    tests: ["test/corpus.test.ts", "-t", "^cloned_counted_adapters.rs$"],
  },
  {
    name: "taking-length-its-source",
    breaks: "an explicit `take`'s length is its source's, not the fewer of that and how many it takes",
    file: "src/runtime/taking.js",
    find: "  taking.len = () => Math.min(taking.iter.len(), taking.n);\n",
    replace: "  taking.len = () => taking.iter.len();\n",
    tests: ["test/corpus.test.ts", "-t", "^cloned_counted_adapters.rs$"],
  },
  {
    name: "taking-front-uncounted",
    breaks: "an explicit `take` stepped from its front never stops",
    file: "src/runtime/taking.js",
    find: "      taking.n--;\n      return taking.iter.next();\n",
    replace: "      return taking.iter.next();\n",
    tests: ["test/corpus.test.ts", "-t", "^cloned_counted_adapters.rs$"],
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
    find: "      const folded = taking.n === 0 || !trimmed() ? init : $foldBack(taking.iter, init, f);\n",
    replace: "      const folded = taking.n === 0 ? init : $foldBack(taking.iter, init, f);\n",
    tests: ["test/corpus.test.ts", "-t", "^own_back_through_adapters.rs$"],
  },
  {
    name: "taking-nth-back-unasked",
    breaks: "a `take`'s `nth_back` steps by `next_back`, not its source's `nth_back` (#220)",
    file: "src/runtime/taking.js",
    find: "    taking.nthBack = (k) => {\n",
    replace: "    taking.unused = (k) => {\n",
    tests: ["test/corpus.test.ts", "-t", "^own_back_through_adapters.rs$"],
  },
  {
    name: "taking-back-end-kept",
    breaks: "a `take` ended from its back isn't done with what it steps (#225)",
    file: "src/runtime/taking.js",
    find: "    if (taking.n === 0) return $closed(taking.iter);\n",
    replace: "    if (taking.n === 0) return { done: true, value: undefined };\n",
    tests: ["test/corpus.test.ts", "-t", "^own_iterator_dropped_through_adapters.rs$"],
  },
];
