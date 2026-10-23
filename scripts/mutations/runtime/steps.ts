// Mutations of src/runtime/steps.js (#161, ADR 0669): each adapter of an
// iterator that may give `None` and then more stays ended, as JS's do.
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "steps-map-ended",
    breaks: "a `map` of an iterator that gives `None` then more stays ended, JS's helper",
    file: "src/runtime/steps.js",
    find: "  map(f) {\n",
    replace: "  mapStepped(f) {\n",
    tests: ["test/corpus.test.ts", "-t", "^restarting_iterators.rs$"],
  },
  {
    name: "steps-filter-ended",
    breaks: "a `filter` of an iterator that gives `None` then more stays ended",
    file: "src/runtime/steps.js",
    find: "  filter(keep) {\n",
    replace: "  filterStepped(keep) {\n",
    tests: ["test/corpus.test.ts", "-t", "^restarting_iterators.rs$"],
  },
  {
    name: "steps-take-counts-no-none",
    breaks: "`take(n)` of an iterator that gives `None` then more counts only its items, as JS's does, not each step, as std's does",
    file: "src/runtime/steps.js",
    find: "  take(n) {\n",
    replace: "  takeStepped(n) {\n",
    tests: ["test/corpus.test.ts", "-t", "^restarting_iterators.rs$"],
  },
  {
    name: "steps-drop-skips-past-none",
    breaks: "`skip(n)` of an iterator that gives `None` then more skips past the `None`",
    file: "src/runtime/steps.js",
    find: "          if (step.done) {\n            n = 0;\n            return step;\n          }\n",
    replace: "          if (step.done) {\n            return step;\n          }\n",
    tests: ["test/corpus.test.ts", "-t", "^restarting_iterators.rs$"],
  },
];
