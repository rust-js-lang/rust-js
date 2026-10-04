// Mutations of src/runtime/parse_big.js (ADR 0093).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "parse-big-overflow-before-digit",
    breaks: "an `i64`'s `\"99999999999999999999x\"`-like parse reports what it overflowed before the digit Rust reads first",
    file: "src/runtime/parse_big.js",
    find: "    const digit = /^[0-9a-z]$/i.test(c) ? parseInt(c, 36) : radix;\n    if (digit >= radix) return error(\"invalid digit found in string\");\n    n *= BigInt(radix);\n    if (negative ? n < min : n > max) return overflow();\n",
    replace: "    n *= BigInt(radix);\n    if (negative ? n < min : n > max) return overflow();\n    const digit = /^[0-9a-z]$/i.test(c) ? parseInt(c, 36) : radix;\n    if (digit >= radix) return error(\"invalid digit found in string\");\n",
    tests: ["test/corpus.test.ts", "-t", "parse_error_order"],
  },
];
