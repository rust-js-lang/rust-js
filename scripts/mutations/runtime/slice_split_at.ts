// Mutations of crates/rust-js-program/src/runtime/slice_split_at.js.
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "slice-split-at-unchecked",
    breaks: "`v.split_at(mid)` past the end of a slice doesn't panic",
    file: "crates/rust-js-program/src/runtime/slice_split_at.js",
    find: '  if (mid > items.length) throw $panic("mid > len", raisedAt);\n',
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "slice_split_at_panic"],
  },
  {
    name: "slice-split-at-checked-panics",
    breaks: "`v.split_at_checked(mid)` past the end of a slice panics, not `None`",
    file: "crates/rust-js-program/src/runtime/slice_split_at.js",
    find: "  return mid > items.length ? undefined : [$part(items, 0, mid), $part(items, mid)];",
    replace: "  return [$part(items, 0, mid), $part(items, mid)];",
    tests: ["test/corpus.test.ts", "-t", "slice_split.rs"],
  },
];
