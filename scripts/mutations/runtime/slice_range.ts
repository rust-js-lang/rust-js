// Mutations of crates/rust-js-program/src/runtime/slice_range.js (ADR 0063).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "slice-start-past-end-unchecked",
    breaks: "`&v[a..]` with `a` past the end is empty, not a panic",
    file: "crates/rust-js-program/src/runtime/slice_range.js",
    find: "function $slice(items, start, raisedAt, end = items.length, inclusive) {\n  if (start > end || end > items.length) $sliceIndexFail(start, end, items.length, raisedAt, inclusive);\n",
    replace: "function $slice(items, start, raisedAt, end = items.length, inclusive) {\n  if (end > items.length) $sliceIndexFail(start, end, items.length, raisedAt, inclusive);\n",
    tests: ["test/corpus.test.ts","-t","slice_from_past_len"],
  },
  {
    name: "owned-copy-placed",
    breaks: "an owned copy of a range, `v[1..3].to_vec()`, says it's in `v`, so its pointer is `v`'s and reads what `v` holds, not what was written to it",
    file: "crates/rust-js-program/src/runtime/slice_range.js",
    find: "  return items.slice(start, end);\n}\n\n// A deque's `range",
    replace: "  return $part(items, start, end);\n}\n\n// A deque's `range",
    tests: ["test/corpus.test.ts", "-t", "^slice_parts_by_address.rs$"],
  },
  {
    name: "slice-range-start-first",
    breaks: "a deque's `range(5..7)` of three items says \"range start index 5\", checked as `&v[a..b]` is, where std's `slice::range` says \"range end index 7\" (#1937)",
    file: "crates/rust-js-program/src/runtime/slice_range.js",
    find: "  $checkRange(start, end, items.length, raisedAt, inclusive);\n  return $part(items, start, end);\n",
    replace: "  if (start > end || end > items.length) $sliceIndexFail(start, end, items.length, raisedAt, inclusive);\n  return $part(items, start, end);\n",
    tests: ["test/corpus.test.ts", "-t", "^deque_range_checked_end_first.rs$"],
  },
];
