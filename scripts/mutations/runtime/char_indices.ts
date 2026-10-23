// Mutations of src/runtime/char_indices.js (ADR 0093).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "char-indices-units",
    breaks: "`char_indices()` counts UTF-16 units, not UTF-8 bytes",
    file: "src/runtime/char_indices.js",
    find: "    at += $byteLen(c);",
    replace: "    at += c.length;",
    tests: ["test/corpus.test.ts", "-t", "string_bytes"],
  },
  {
    name: "char-indices-offset-end-short",
    breaks: "offset past the last char is the string's length",
    file: "src/runtime/char_indices.js",
    find: "last[0] + $byteLen(last[1])",
    replace: "last[0]",
    tests: ["test/corpus.test.ts", "-t", "str_odds"],
  },
];
