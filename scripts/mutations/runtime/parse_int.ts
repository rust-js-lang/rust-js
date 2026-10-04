// Mutations of src/runtime/parse_int.js (ADR 0093).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "parse-overflow-before-digit",
    breaks: "`\"26x\".parse::<u8>()` reports the overflow of 260, where Rust reads the `x` first",
    file: "src/runtime/parse_int.js",
    find: "    const digit = /^[0-9a-z]$/i.test(c) ? parseInt(c, 36) : radix;\n    if (digit >= radix) return error(\"invalid digit found in string\");\n    n *= radix;\n    if (negative ? n < min : n > max) return overflow();\n",
    replace: "    n *= radix;\n    if (negative ? n < min : n > max) return overflow();\n    const digit = /^[0-9a-z]$/i.test(c) ? parseInt(c, 36) : radix;\n    if (digit >= radix) return error(\"invalid digit found in string\");\n",
    tests: ["test/corpus.test.ts", "-t", "parse_error_order"],
  },
];
