// Mutations of src/runtime/slice_starts_with.js.
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "slice-ends-with-from-start",
    breaks: "`v.ends_with(s)` of a slice compares its first items, not its last",
    file: "src/runtime/slice_starts_with.js",
    find: "  const at = end ? items.length - prefix.length : 0;",
    replace: "  const at = 0;",
    tests: ["test/corpus.test.ts", "-t", "slice_prefixes"],
  },
  {
    name: "slice-starts-with-longer",
    breaks: "`v.starts_with(s)` of a slice shorter than `s` is true where its items match",
    file: "src/runtime/slice_starts_with.js",
    find: "  return at >= 0 && prefix.length <= items.length && prefix.every((item, i) => eq(items[at + i], item));",
    replace: "  return at >= 0 && prefix.every((item, i) => eq(items[at + i], item) || items[at + i] === undefined);",
    tests: ["test/corpus.test.ts", "-t", "slice_prefixes"],
  },
  {
    name: "starts-with-eq-ignored",
    breaks: "a slice's `starts_with` compares by `===`, not the items' `==`",
    file: "src/runtime/slice_starts_with.js",
    find: "prefix.every((item, i) => eq(items[at + i], item))",
    replace: "prefix.every((item, i) => items[at + i] === item)",
    tests: ["test/corpus.test.ts","-t","slice_prefix_eq"],
  },
];
