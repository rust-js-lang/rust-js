// Mutations of src/runtime/range_from_next.js (ADR 0129, #414).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "range-from-unwrapped",
    breaks: "an `a..` of integers steps past its type's last, `255u8..` giving `256`, `i32::MAX..` past it (#414)",
    file: "src/runtime/range_from_next.js",
    find: "    range.start = n === hi ? lo : typeof n === \"bigint\" ? n + 1n : n + 1;\n",
    replace: "    range.start = typeof n === \"bigint\" ? n + 1n : n + 1;\n",
    tests: ["test/corpus.test.ts", "-t", "^range_from_wraps.rs$"],
  },
  {
    name: "range-from-char-unchecked",
    breaks: "an `a..` of `char`s steps past `char::MAX` to nothing, where std panics `overflow in `Step::forward``",
    file: "src/runtime/range_from_next.js",
    find: "    if (next === undefined) throw new Error(\"overflow in `Step::forward`\");\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^range_from_wraps.rs$"],
  },
];
