// Mutations of src/runtime/position.js (ADR 0093).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    "name": "rposition-from-front",
    "breaks": "`rposition` tests from the front and finds the first",
    "file": "src/runtime/position.js",
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
    "file": "src/runtime/position.js",
    "find": "      if (iter) items.end = i;\n",
    "replace": "",
    "tests": [
      "test/corpus.test.ts",
      "-t",
      "rposition"
    ]
  },
  {
    name: "position-by-next",
    breaks: "position is its iterator's tryFold",
    file: "src/runtime/position.js",
    find: "  const hit = $lent($stepsOf(items)).tryFold(",
    replace: "  const hit = new $Steps((it => () => it.next())($stepsOf(items))).tryFold(",
    tests: ["test/corpus.test.ts", "-t", "skip_try_fold"],
  },
];
