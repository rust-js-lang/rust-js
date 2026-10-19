// Mutations of src/runtime/back_steps.js (ADR 0093).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "nth-back-off-by-one",
    breaks: "`nth_back(n)` gives the item after the `n`th from the back",
    file: "src/runtime/back_steps.js",
    find: "    if (n-- === 0) return found(step.value);\n",
    replace: "    if (--n === 0) return found(step.value);\n",
    tests: ["test/corpus.test.ts", "-t", "^back_steps.rs$"],
  },
  {
    name: "fold-back-from-front",
    breaks: "`rfold` folds from the front",
    file: "src/runtime/back_steps.js",
    find: "  for (let step = it.back(); !step.done; step = it.back()) folded = f(folded, step.value);\n",
    replace: "  for (const item of it) folded = f(folded, item);\n",
    tests: ["test/corpus.test.ts", "-t", "^back_steps.rs$"],
  },
  {
    name: "find-back-unfound",
    breaks: "`rfind` gives what it found without its `Some`, where items could look like `None`",
    file: "src/runtime/back_steps.js",
    find: "    if (p(step.value)) return found(step.value);\n",
    replace: "    if (p(step.value)) return step.value;\n",
    tests: ["test/corpus.test.ts", "-t", "^back_steps.rs$"],
  },
  {
    name: "find-back-drops-kept",
    breaks: "`rfind` drops none of the items it passes over",
    file: "src/runtime/back_steps.js",
    find: "    if (p(step.value)) return found(step.value);\n    drop?.(step.value);\n",
    replace: "    if (p(step.value)) return found(step.value);\n",
    tests: ["test/corpus.test.ts", "-t", "^back_steps_drop.rs$"],
  },
  {
    name: "map-back-unmapped",
    breaks: "a map stepped from its back gives its source's items, not their `f`'s",
    file: "src/runtime/back_steps.js",
    find: "  map.back = () => mapped(it.back());\n",
    replace: "  map.back = () => it.back();\n",
    tests: ["test/corpus.test.ts", "-t", "^back_steps.rs$"],
  },
];
