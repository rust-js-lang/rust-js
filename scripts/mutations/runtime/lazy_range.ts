// Mutations of src/runtime/lazy_range.js (#163).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "lazy-range-step-unread",
    breaks: "a range's numbers `step` apart are each number, `step_by` given no effect",
    file: "src/runtime/lazy_range.js",
    find: "    at = Number(at) + step;\n    return { done: false, value };",
    replace: "    at = Number(at) + 1;\n    return { done: false, value };",
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
  {
    name: "lazy-range-nth-stepped",
    breaks: "a lazy range has no `nth` of its bounds, so a `step_by` of one given to generic code steps each number and doesn't finish (#231)",
    file: "src/runtime/lazy_range.js",
    find: "  range.nth = (k) => {\n",
    replace: "  range.unused = (k) => {\n",
    tests: ["test/corpus.test.ts", "-t", "^range_steps_anywhere.rs$"],
  },
  {
    name: "lazy-range-hint-unknown",
    breaks: "a range written in place, given to generic code, says nothing of how many it has left, `(0, None)` where Rust's is exact (ADR 0918)",
    file: "src/runtime/lazy_range.js",
    find: "    return [left, left];\n  };\n  return range;\n",
    replace: "    return [0, undefined];\n  };\n  return range;\n",
    tests: ["test/corpus.test.ts", "-t", "^range_hints_given.rs$"],
  },
];
