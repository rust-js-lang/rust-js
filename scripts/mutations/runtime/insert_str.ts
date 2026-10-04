// Mutations of src/runtime/insert_str.js (ADR 0093).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "insert-inside-char",
    breaks: "`s.insert(3, 'y')` inside an `é` splits it, where Rust's assertion fails",
    file: "src/runtime/insert_str.js",
    find: '  if (unit === undefined) throw new Error("assertion failed: self.is_char_boundary(idx)");\n',
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "string_insert_boundary"],
  },
];
