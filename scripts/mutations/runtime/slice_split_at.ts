// Mutations of src/runtime/slice_split_at.js.
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "slice-split-at-unchecked",
    breaks: "`v.split_at(mid)` past the end of a slice doesn't panic",
    file: "src/runtime/slice_split_at.js",
    find: '  if (mid > items.length) throw new Error("mid > len");\n',
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "slice_split_at_panic"],
  },
];
