// Mutations of crates/rust-js-program/src/runtime/position.js (ADR 0093).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    "name": "rposition-from-front",
    "breaks": "`rposition` tests from the front and finds the first",
    "file": "crates/rust-js-program/src/runtime/position.js",
    "find": "  for (let i = end - 1; i >= at; i--) {\n",
    "replace": "  for (let i = at; i < end; i++) {\n",
    "tests": [
      "test/corpus.test.ts",
      "-t",
      "rposition"
    ]
  },
  {
    "name": "rposition-leaves-kept-whole",
    "breaks": "`rposition` of a kept iterator leaves what it stepped from its back there",
    "file": "crates/rust-js-program/src/runtime/position.js",
    "find": "    if (iter) items.end = i;\n    if (found(array[i])) return i - at;\n",
    "replace": "    if (found(array[i])) return i - at;\n",
    "tests": [
      "test/corpus.test.ts",
      "-t",
      "rposition"
    ]
  },
  {
    name: "rposition-takes-after-asking",
    breaks: "`rposition` of a `$iter` takes each item only once `f` has given its answer, so one `f` owns and drops as it panics is dropped again with what's left (#738)",
    file: "crates/rust-js-program/src/runtime/position.js",
    find: "    if (iter) items.end = i;\n    if (found(array[i])) return i - at;\n",
    replace: "    if (found(array[i])) return i - at;\n    if (iter) items.end = i;\n",
    tests: ["test/corpus.test.ts", "-t", "^back_predicate_unwinds.rs$"],
  },
  {
    name: "position-by-next",
    breaks: "position is its iterator's tryFold",
    file: "crates/rust-js-program/src/runtime/position.js",
    find: "  const hit = $lent($stepsOf(items)).tryFold(",
    replace: "  const hit = new $Steps((it => () => it.next())($stepsOf(items))).tryFold(",
    tests: ["test/corpus.test.ts", "-t", "skip_try_fold"],
  },
  {
    name: "rposition-of-a-js-iterator-by-next-back",
    breaks: "`rposition` of a JS iterator steps its `next_back` one at a time, never its own `try_rfold`, so an `enumerate()`'s asks its source's length at each step where std's asks it once (#2142)",
    file: "crates/rust-js-program/src/runtime/position.js",
    find: "    const hit = $tryRfoldOf(items, false, (_, item) => (i--, found(item)), (hit) => hit);\n    return hit ? i : undefined;\n",
    replace: "    for (let step = items.back(); !step.done; step = items.back()) {\n      i--;\n      if (found(step.value)) return i;\n    }\n    return undefined;\n",
    tests: ["test/corpus.test.ts", "test/iterator-steps.test.ts", "-t", "^rposition_asks_try_rfold.rs$|asks what it holds as std's"],
  },
];
