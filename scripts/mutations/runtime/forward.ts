// Mutations of src/runtime/forward.js (ADR 1089).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "forward-arguments-dropped",
    breaks: "a step passed on is given only its first two arguments: a `rev()`'s `fold`, its source's `rfold`, loses the `unwind` that drops the accumulator a panic strands, and a lent iterator's `try_fold` its `broke`'s `unwind` (#423)",
    file: "src/runtime/forward.js",
    find: "          ? (...args) => source[by](...args)\n",
    replace: "          ? (a, b) => source[by](a, b)\n",
    tests: ["test/corpus.test.ts", "-t", "^panic_in_own_rfold_drops.rs$"],
  },
  {
    name: "forward-own-unsaid",
    breaks: "a step passed on isn't said to be its source's own: a `rev()` of a `Vec`'s `IntoIter` doesn't say its `nth` drops what it passes over, so a `skip` of it drops each one from the back, one by one (#234)",
    file: "src/runtime/forward.js",
    find: "      if (each === undefined && source[$ownStep(by)]) adapter[$ownStep(name)] = true;\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^vec_skipped_from_back_drops.rs$"],
  },
  {
    name: "forward-step-it-lacks",
    breaks: "a step is passed on to a source that hasn't it: a `map()` of a slice's items, stepped from its back, folds by calling the `rfold` they haven't",
    file: "src/runtime/forward.js",
    find: "    } else if (by !== undefined && source[by] !== undefined) {\n",
    replace: "    } else if (by !== undefined) {\n",
    tests: ["test/corpus.test.ts", "-t", "^(back_steps|back_stepped_adapter_sources|filter_ends).rs$"],
  },
];
