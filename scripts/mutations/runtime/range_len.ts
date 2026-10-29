// Mutations of src/runtime/range_len.js (ADR 0093).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "range-len-past-end",
    breaks: "a range's `len()` past its end is negative, not zero",
    file: "src/runtime/range_len.js",
    find: "  return range.start < range.end ? range.end - range.start : 0;\n",
    replace: "  return range.end - range.start;\n",
    tests: ["test/corpus.test.ts", "-t", "^exact_size_len.rs$"],
  },
  {
    name: "range-nth-start-unmoved",
    breaks: "a range's `nth(n)` leaves its start where it was, so its next `next()` gives a number again",
    file: "src/runtime/range_len.js",
    find: "  if (at < range.end) {\n    range.start = big ? at + 1n : at + 1;\n",
    replace: "  if (at < range.end) {\n",
    tests: ["test/corpus.test.ts", "-t", "^range_own_steps.rs$"],
  },
  {
    name: "range-nth-back-end-unmoved",
    breaks: "a reversed range's `nth(n)` leaves its end where it was, so its next `next()` gives the same numbers again",
    file: "src/runtime/range_len.js",
    find: "  if (at >= range.start) {\n    range.end = at;\n",
    replace: "  if (at >= range.start) {\n",
    tests: ["test/corpus.test.ts", "-t", "^range_own_steps.rs$"],
  },
  {
    name: "range-inclusive-nth-back-start-excluded",
    breaks: "a reversed `a..=b`'s `nth(n)` that lands on its start `a` gives none",
    file: "src/runtime/range_len.js",
    find: "  const at = big ? range.end - BigInt(n) : range.end - n;\n  if (at >= range.start) {\n",
    replace: "  const at = big ? range.end - BigInt(n) : range.end - n;\n  if (at > range.start) {\n",
    tests: ["test/corpus.test.ts", "-t", "^range_own_steps.rs$"],
  },
  {
    name: "range-inclusive-len-exclusive",
    breaks: "`(a..=b).count()` counts one fewer, as `a..b`'s",
    file: "src/runtime/range_len.js",
    find: "  return range.start <= range.end ? range.end - range.start + 1 : 0;\n",
    replace: "  return range.start <= range.end ? range.end - range.start : 0;\n",
    tests: ["test/corpus.test.ts", "-t", "^range_own_steps.rs$"],
  },
  {
    name: "range-from-unhinted",
    breaks: "a zip of `n..`, `(0..).zip(w)`, hints `(0, Some(2))`, as `n..` hints nothing, not `usize::MAX` (#400)",
    file: "src/runtime/range_len.js",
    find: "  if (next === $rangeFromNext) return () => [4294967295, undefined];\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^zip_size_hints.rs$"],
  },
  {
    name: "size-hint-of-range-unknown",
    breaks: "a range given as an `Iterator`, `{ next: $rangeNext }`, hints `(0, None)`, so std's `Peekable` of one does, and its `len()` panics",
    file: "src/runtime/range_len.js",
    find: "    next === $rangeNext || next === $rangeNextBack\n",
    replace: "    false\n",
    tests: ["test/corpus.test.ts", "-t", "^peekable_range_len.rs$"],
  },
  {
    name: "inclusive-range-hint-unknown",
    breaks: "an `a..=b` given as it is to std's `Skip` says nothing of how many it has left, so its `len` asserts and panics (ADR 0918)",
    file: "src/runtime/range_len.js",
    find: "      : next === $rangeInclusiveNext || next === $rangeInclusiveNextBack\n",
    replace: "      : false\n",
    tests: ["test/iterator-forms.test.ts", "-t", "^range_hints_given with std's iterator structs$"],
  },
];
