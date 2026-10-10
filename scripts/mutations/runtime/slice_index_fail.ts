// Mutations of src/runtime/slice_index_fail.js (ADR 0063).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "slice-index-fail-end-first",
    breaks: "`&v[a..b]` past the end says its end is out of range, not its start",
    file: "src/runtime/slice_index_fail.js",
    find: "  if (start > length) throw new Error(`range start index ${start} out of range for slice of length ${length}`);\n  if (end > length) throw new Error(`range end index ${end} out of range for slice of length ${length}`);\n",
    replace: "  if (end > length) throw new Error(`range end index ${end} out of range for slice of length ${length}`);\n  if (start > length) throw new Error(`range start index ${start} out of range for slice of length ${length}`);\n",
    tests: ["test/corpus.test.ts","-t","slice_start_past_len"],
  },
  {
    name: "check-range-start-first",
    breaks: "`slice::range` checks its start before its end, as indexing does",
    file: "src/runtime/slice_index_fail.js",
    find: "  if (end > length) $sliceIndexFail(0, end, length);\n  if (start > end) $sliceIndexFail(start, end, length);\n",
    replace: "  if (start > end || end > length) $sliceIndexFail(start, end, length);\n",
    tests: ["test/corpus.test.ts","-t","extend_within_past_len"],
  },
];
