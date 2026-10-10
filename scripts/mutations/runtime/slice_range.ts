// Mutations of src/runtime/slice_range.js (ADR 0063).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "slice-start-past-end-unchecked",
    breaks: "`&v[a..]` with `a` past the end is empty, not a panic",
    file: "src/runtime/slice_range.js",
    find: "  if (start > end || end > items.length) $sliceIndexFail(start, end, items.length);\n",
    replace: "  if (end > items.length) $sliceIndexFail(start, end, items.length);\n",
    tests: ["test/corpus.test.ts","-t","slice_from_past_len"],
  },
];
