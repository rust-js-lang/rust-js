// Mutations of src/runtime/slice_split_at.js.
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "slice-split-at-unchecked",
    breaks: "`v.split_at(mid)` past the end of a slice doesn't panic",
    file: "src/runtime/slice_split_at.js",
    find: '    throw new Error("mid > len");\n',
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "slice_split_at_panic"],
  },
  {
    name: "slice-split-at-checked-panics",
    breaks: "`v.split_at_checked(mid)` past the end of a slice panics, not `None`",
    file: "src/runtime/slice_split_at.js",
    find: "    if (checked) return undefined;\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "slice_split.rs"],
  },
];
