// Mutations of src/runtime/lazy_range.js (#163).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "lazy-range-step-unread",
    breaks: "a range's numbers `step` apart are each number, `step_by` given no effect",
    file: "src/runtime/lazy_range.js",
    find: "  for (let i = start; i < end; i = Number(i) + step) {",
    replace: "  for (let i = start; i < end; i++) {",
    tests: ["test/corpus.test.ts", "-t", "^range_steps_known.rs$"],
  },
  {
    name: "lazy-range-len-floor",
    breaks: "how many numbers a range `step` apart has rounds down, missing the last",
    file: "src/runtime/lazy_range.js",
    find: "Math.ceil((end - start) / step)",
    replace: "Math.floor((end - start) / step)",
    tests: ["test/corpus.test.ts", "-t", "^range_steps_known.rs$"],
  },
];
