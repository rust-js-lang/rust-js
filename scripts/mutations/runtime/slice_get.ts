// Mutations of src/runtime/slice_get.js.
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "slice-get-backwards",
    breaks: "`v.get(3..1)` of a slice is `Some` of nothing, where `&v[3..1]` would panic",
    file: "src/runtime/slice_get.js",
    find: "  return start > end || end > items.length ? undefined : items.slice(start, end);",
    replace: "  return end > items.length ? undefined : items.slice(start, end);",
    tests: ["test/corpus.test.ts", "-t", "slice_split"],
  },
  {
    name: "slice-get-past-end",
    breaks: "`v.get(4..9)` of a slice is `Some` of what there is, where `&v[4..9]` would panic",
    file: "src/runtime/slice_get.js",
    find: "  return start > end || end > items.length ? undefined : items.slice(start, end);",
    replace: "  return start > end ? undefined : items.slice(start, end);",
    tests: ["test/corpus.test.ts", "-t", "slice_split"],
  },
];
