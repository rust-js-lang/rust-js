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
    name: "lazy-range-back-inclusive-short",
    breaks: "a reversed `a..=b`'s `step_by` starts below its end `b`, as `a..b`'s does",
    file: "src/runtime/lazy_range.js",
    find: "  for (let i = inclusive ? end : end - (big ? 1n : 1); i >= start; i -= by) {\n",
    replace: "  for (let i = end - (big ? 1n : 1); i >= start; i -= by) {\n",
    tests: ["test/corpus.test.ts", "-t", "^range_steps_known.rs$"],
  },
];
