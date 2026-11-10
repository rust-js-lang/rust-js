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
    find: "  let at = chars ? (inclusive ? end : $charStep(end, -1)) : inclusive ? end : end - (big ? 1n : 1);\n",
    replace: "  let at = chars ? (inclusive ? end : $charStep(end, -1)) : end - (big ? 1n : 1);\n",
    tests: ["test/corpus.test.ts", "-t", "^range_steps_known.rs$"],
  },
  {
    name: "lazy-range-hint-unknown",
    breaks: "a range written in place, given to generic code, says nothing of how many it has left, `(0, None)` where Rust's is exact (ADR 0918)",
    file: "src/runtime/lazy_range.js",
    find: "    return [left, left];\n  };\n  return range;\n",
    replace: "    return [0, undefined];\n  };\n  return range;\n",
    tests: ["test/corpus.test.ts", "-t", "^range_hints_given.rs$"],
  },
  {
    name: "lazy-range-back-chars-as-numbers",
    breaks: "a reversed `char` range's `step_by(n)` steps by subtraction, `NaN` after its first, `('a'..='e').rev().step_by(2)` giving `\"e\"`, not `\"eca\"` (#420)",
    file: "src/runtime/lazy_range.js",
    find: "  const chars = typeof end === \"string\";\n",
    replace: "  const chars = false;\n",
    tests: ["test/corpus.test.ts", "-t", "^char_ranges_stepped.rs$"],
  },
  {
    name: "lazy-range-back-unhinted",
    breaks: "a reversed range's `step_by` held, `let it = (0..n).rev().step_by(2)`, hints nothing, so its `collect()` of `usize::MAX` asks no room and throws JS's `RangeError` where std panics `capacity overflow` (#1046)",
    file: "src/runtime/lazy_range.js",
    find: "  range.sizeHint = () => {\n    if (ended()) return [0, 0];\n",
    replace: "  range.unhinted = () => {\n    if (ended()) return [0, 0];\n",
    tests: ["test/corpus.test.ts", "-t", "^counted_iterators_held_ask_room.rs$"],
  },
  {
    name: "lazy-range-back-hint-counted-apart",
    breaks: "a reversed range's `step_by` hints its range's count, not that many `step` apart, `(0..10).rev().step_by(3)`'s `(10, Some(10))`, not `(4, Some(4))` (#1046)",
    file: "src/runtime/lazy_range.js",
    find: "    const left = (n) => (n === 0 ? 0 : 1 + Math.floor((n - 1) / step));\n",
    replace: "    const left = (n) => n;\n",
    tests: ["test/corpus.test.ts", "-t", "^counted_iterators_held_ask_room.rs$"],
  },
  {
    name: "lazy-range-advance-by-steps",
    breaks: "a generic `skip(4_000_000_000)` of a chain of `0..u32::MAX` steps each number, `$lazyRange` having no `advance_by` of its bounds, where std's `Range::advance_by` ends at once (#1240)",
    file: "src/runtime/lazy_range.js",
    find: "      if (passed > 0) at = Number(at) + passed * step;\n      return m - passed;\n",
    replace: "      for (let k = 0; k < passed; k++) at = Number(at) + step;\n      return m - passed;\n",
    tests: ["test/corpus.test.ts", "-t", "^range_advanced_by_its_bounds\\.rs$"],
  },
  {
    name: "lazy-range-advanced-each",
    breaks: "a lazy range has no `advance_by` of its bounds, so its `nth`, std's default of it, given to generic code steps each number and doesn't finish (#231, #1240)",
    file: "src/runtime/lazy_range.js",
    find: "    advanceBy: (m) => {\n",
    replace: "    unused: (m) => {\n",
    tests: ["test/corpus.test.ts", "-t", "^lazy_range_nth_given_to_generic_code.rs$"],
  },
];
