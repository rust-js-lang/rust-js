// Mutations of src/runtime/insert_at.js (ADR 0093).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "insert-past-end-leaks",
    breaks:
      "an `insert` past a `Vec`'s end never drops what it took as it panics: `drop 9` missing where Rust drops it (#1171)",
    file: "src/runtime/insert_at.js",
    find: "    drop?.(item);\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^edits_that_panic_keep_what_std_keeps.rs$"],
  },
];
