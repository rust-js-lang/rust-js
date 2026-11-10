// Mutations of src/runtime/slice_range.js (ADR 0063).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "slice-start-past-end-unchecked",
    breaks: "`&v[a..]` with `a` past the end is empty, not a panic",
    file: "src/runtime/slice_range.js",
    find: "function $slice(items, start, end = items.length) {\n  if (start > end || end > items.length) $sliceIndexFail(start, end, items.length);\n",
    replace: "function $slice(items, start, end = items.length) {\n  if (end > items.length) $sliceIndexFail(start, end, items.length);\n",
    tests: ["test/corpus.test.ts","-t","slice_from_past_len"],
  },
  {
    name: "owned-copy-placed",
    breaks: "an owned copy of a range, `v[1..3].to_vec()`, says it's in `v`, so its pointer is `v`'s and reads what `v` holds, not what was written to it",
    file: "src/runtime/slice_range.js",
    find: "  return items.slice(start, end);\n}\n\n// `v.extend_from_within",
    replace: "  return $part(items, start, end);\n}\n\n// `v.extend_from_within",
    tests: ["test/corpus.test.ts", "-t", "^slice_parts_by_address.rs$"],
  },
];
