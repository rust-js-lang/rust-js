// Mutations of src/runtime/cow_key.js (ADR 0402).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "cow-key-whole",
    breaks: "a map of `Cow`s keys one by its object, its variant too: `Owned(\"a\")` doesn't find `Borrowed(\"a\")`",
    file: "src/runtime/cow_key.js",
    find: '  return $key(typeof key === "object" && !Array.isArray(key) ? key._0 : key);\n',
    replace: "  return $key(key);\n",
    tests: ["test/corpus.test.ts", "-t", "^cow_keys.rs$"],
  },
  {
    name: "cow-key-array-unwrapped",
    breaks: "a `&[T]` asked of a map of `Cow<[T]>`s is taken for a `Cow`, its `_0` read",
    file: "src/runtime/cow_key.js",
    find: '  return $key(typeof key === "object" && !Array.isArray(key) ? key._0 : key);\n',
    replace: '  return $key(typeof key === "object" ? key._0 : key);\n',
    tests: ["test/corpus.test.ts", "-t", "^cow_keys.rs$"],
  },
];
