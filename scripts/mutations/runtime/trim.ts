// Mutations of src/runtime/trim.js.
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "trim-by-js-whitespace",
    breaks: "`trim()` keeps U+0085 and trims U+FEFF, as JS's does, where Rust's does the other",
    file: "src/runtime/trim.js",
    find: "  return $trimEnd($trimStart(s));",
    replace: "  return s.trim();",
    tests: ["test/corpus.test.ts", "-t", "code_point_order"],
  },
];
